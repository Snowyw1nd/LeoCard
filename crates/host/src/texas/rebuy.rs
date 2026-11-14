use super::TexasSession;
use crate::features::MatchSettlement;
use crate::lifecycle::HostedGameLifecycle;
use crate::{ConnectionId, Delivery};
use leocard_game_common::PlayerId;
use leocard_protocol::{
    GameViolation, PlayerViolation, RejectReason, RequestId, TexasRebuyChoice, TexasRebuyView,
    TexasViolation,
};
use std::time::{Duration, Instant};

const REBUY_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone, Debug)]
pub(super) struct RebuyWindow {
    hand_number: u32,
    remaining: Duration,
    deadline: Instant,
    choices: Vec<TexasRebuyChoice>,
}

impl RebuyWindow {
    fn remaining(&self) -> Duration {
        self.remaining
            .min(self.deadline.saturating_duration_since(Instant::now()))
    }

    fn remaining_ms(&self) -> u32 {
        self.remaining().as_millis().div_ceil(1000) as u32 * 1000
    }
}

impl TexasSession {
    pub(super) fn open_rebuy_window(&mut self) {
        let Some(game) = self.game.as_ref().filter(|game| game.rules().rebuy) else {
            return;
        };
        // 已离线或离场的真人同样不能参与继续比赛的补码批次。
        if game.players().iter().any(|view| {
            !self.room.players.iter().any(|player| {
                player.id == view.id && !player.left && (player.connected || player.is_bot)
            })
        }) {
            return;
        }
        let cleared = game
            .game()
            .players()
            .iter()
            .zip(game.players())
            .filter(|(state, _)| state.stack() == 0)
            .map(|(state, player)| (state.id(), player.id))
            .collect::<Vec<_>>();
        if cleared.is_empty()
            || cleared.iter().any(|(core, id)| {
                game.game().rebought_chips(*core) != 0
                    || !self.room.players.iter().any(|player| {
                        player.id == *id
                            && player.connected
                            && !player.left
                            && !player.is_bot
                            && player.game_profiles.coins > 0
                    })
            })
        {
            return;
        }
        self.rebuy = Some(RebuyWindow {
            hand_number: game.game().hand_number(),
            remaining: REBUY_TIMEOUT,
            deadline: Instant::now() + REBUY_TIMEOUT,
            choices: cleared
                .into_iter()
                .map(|(_, player)| TexasRebuyChoice {
                    player,
                    chips: None,
                })
                .collect(),
        });
    }

    pub(super) fn rebuy_view(&self) -> Option<TexasRebuyView> {
        self.rebuy.as_ref().map(|window| TexasRebuyView {
            hand_number: window.hand_number,
            remaining_ms: window.remaining_ms(),
            maximum_chips: u32::from(self.rules.starting_chips / 2),
            choices: window.choices.clone(),
        })
    }

    pub(super) fn abort_rebuy(&mut self) -> bool {
        let Some(window) = self.rebuy.take() else {
            return false;
        };
        let decisions = window
            .choices
            .iter()
            .map(|choice| (choice.player, None))
            .collect::<Vec<_>>();
        self.record_rebuy_receipts(&decisions);
        self.commit_match(&());
        true
    }

    pub(super) fn advance_rebuy(&mut self, elapsed: Duration) -> Vec<Delivery> {
        let Some(window) = self.rebuy.as_mut() else {
            return Vec::new();
        };
        let before = window.remaining.as_millis().div_ceil(1000);
        window.remaining = window
            .remaining
            .saturating_sub(elapsed)
            .min(window.deadline.saturating_duration_since(Instant::now()));
        let expired = window.remaining.is_zero();
        let changed = expired || before != window.remaining.as_millis().div_ceil(1000);
        if expired {
            self.abort_rebuy();
        }
        if changed {
            self.room.bump_revision();
            self.broadcast_game(None)
        } else {
            Vec::new()
        }
    }

    pub(super) fn expire_rebuy(&mut self) -> Vec<Delivery> {
        if self
            .rebuy
            .as_ref()
            .is_some_and(|window| window.remaining().is_zero())
        {
            self.abort_rebuy();
            self.room.bump_revision();
            self.broadcast_game(None)
        } else {
            Vec::new()
        }
    }

    pub(super) fn choose_rebuy(
        &mut self,
        connection: ConnectionId,
        request_id: RequestId,
        hand_number: u32,
        chips: Option<u32>,
    ) -> Vec<Delivery> {
        let Some(player) = self.room.player_id(connection) else {
            return self.room.reject(
                connection,
                request_id,
                RejectReason::Player(PlayerViolation::NotJoined),
            );
        };
        let violation = self.validate_rebuy_choice(player, hand_number, chips);
        if let Err(violation) = violation {
            return self.room.reject(
                connection,
                request_id,
                RejectReason::Game(GameViolation::Texas(violation)),
            );
        }
        if let Some(chips) = chips {
            let window = self.rebuy.as_mut().expect("validated pending rebuy");
            window
                .choices
                .iter_mut()
                .find(|choice| choice.player == player)
                .expect("validated rebuy participant")
                .chips = Some(chips);
            if window.choices.iter().all(|choice| choice.chips.is_some()) {
                let purchases = window
                    .choices
                    .iter()
                    .map(|choice| {
                        (
                            choice.player,
                            choice.chips.expect("all rebuy choices are confirmed"),
                        )
                    })
                    .collect::<Vec<_>>();
                let funded = purchases.iter().all(|(id, amount)| {
                    self.room.players.iter().any(|participant| {
                        participant.id == *id
                            && participant.connected
                            && !participant.left
                            && !participant.is_bot
                            && participant.game_profiles.coins >= *amount
                    })
                });
                if funded
                    && self
                        .game
                        .as_mut()
                        .is_some_and(|game| game.rebuy(&purchases).is_ok())
                {
                    let decisions = purchases
                        .iter()
                        .map(|(id, amount)| (*id, Some(*amount)))
                        .collect::<Vec<_>>();
                    self.record_rebuy_receipts(&decisions);
                    for (id, amount) in purchases {
                        let participant = self
                            .room
                            .players
                            .iter_mut()
                            .find(|p| p.id == id)
                            .expect("rebuy participant belongs to the room");
                        participant.game_profiles.coins -= amount;
                    }
                    self.rebuy = None;
                } else {
                    self.abort_rebuy();
                }
            }
        } else {
            self.abort_rebuy();
        }
        self.room.bump_revision();
        self.broadcast_game(Some((connection, request_id)))
    }

    fn validate_rebuy_choice(
        &self,
        player: PlayerId,
        hand_number: u32,
        chips: Option<u32>,
    ) -> Result<(), TexasViolation> {
        let window = self
            .rebuy
            .as_ref()
            .filter(|window| window.hand_number == hand_number && !window.remaining().is_zero())
            .ok_or(TexasViolation::InvalidRebuy)?;
        let choice = window
            .choices
            .iter()
            .find(|choice| choice.player == player)
            .ok_or(TexasViolation::InvalidRebuy)?;
        if choice.chips.is_some() {
            return Err(TexasViolation::RebuyAlreadyChosen);
        }
        if let Some(chips) = chips {
            if chips == 0 || chips > u32::from(self.rules.starting_chips / 2) {
                return Err(TexasViolation::InvalidRebuy);
            }
            if !self.room.players.iter().any(|participant| {
                participant.id == player
                    && participant.connected
                    && !participant.left
                    && !participant.is_bot
                    && participant.game_profiles.coins >= chips
            }) {
                return Err(TexasViolation::InsufficientRebuyCoins);
            }
        }
        Ok(())
    }
}
