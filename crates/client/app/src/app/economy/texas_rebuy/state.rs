use crate::app::runtime::ClientResource;
use crate::app::shell::game_command;
use bevy::prelude::*;
use leocard_client::PlayerEconomy;
use leocard_game_common::PlayerId;
use leocard_protocol::{ClientCommand, GameSnapshot, MatchId, TexasCommand};
use std::collections::{HashSet, VecDeque};

pub(super) struct PendingChoice {
    pub match_id: MatchId,
    pub hand_number: u32,
    pub player: PlayerId,
    pub chips: Option<u32>,
    pub rejection_serial: u64,
}

impl PendingChoice {
    pub(super) fn retained_by(&self, active: &Self) -> bool {
        active.match_id == self.match_id
            && active.hand_number == self.hand_number
            && active.player == self.player
            && (active.chips == self.chips && active.chips.is_some()
                || active.rejection_serial == self.rejection_serial)
    }
}

#[derive(Resource, Default)]
pub(crate) struct TexasRebuyWallet {
    pub(super) pending: Option<PendingChoice>,
    pub(super) commits: VecDeque<(MatchId, u32)>,
    pub(super) committed: HashSet<MatchId>,
    pub(super) cancelled: HashSet<MatchId>,
    pub(super) cancellations: Vec<MatchId>,
    pub(super) retry_after: f32,
    pub(super) last_error: Option<String>,
    pub(super) reservation_warning: bool,
}

impl TexasRebuyWallet {
    pub(crate) fn blocks_rewards(&self) -> bool {
        !self.commits.is_empty() || !self.cancellations.is_empty()
    }

    pub(crate) fn submitted(&self, match_id: MatchId, hand_number: u32) -> bool {
        self.pending.as_ref().is_some_and(|pending| {
            pending.match_id == match_id && pending.hand_number == hand_number
        })
    }

    pub(crate) fn choose(
        &mut self,
        client: &mut ClientResource,
        economy: &mut PlayerEconomy,
        chips: Option<u32>,
    ) -> Result<(), String> {
        let model = client.0.model();
        let Some(GameSnapshot::Texas(game)) = model.game_snapshot() else {
            return Err("当前不在德州牌桌".to_owned());
        };
        let player = model.you().ok_or("无法确认本机玩家身份")?;
        if model.is_spectating() || !game.players.iter().any(|view| view.player.id == player) {
            return Err("旁观者不能补码".to_owned());
        }
        if self.committed.contains(&game.match_id) || self.cancelled.contains(&game.match_id) {
            return Err("补码窗口已结束".to_owned());
        }
        let rebuy = game.rebuy.as_ref().ok_or("补码窗口已结束")?;
        let choice = rebuy
            .choices
            .iter()
            .find(|choice| choice.player == player)
            .ok_or("本次无需补码")?;
        if choice.chips.is_some() || self.submitted(game.match_id, rebuy.hand_number) {
            return Err("已提交补码选择，请等待全体确认".to_owned());
        }
        if self.blocks_rewards() {
            return Err("钱包正在同步，请稍后重试".to_owned());
        }
        let reserved_chips = economy
            .pending_texas_rebuys()
            .iter()
            .find(|(match_id, _)| *match_id == game.match_id)
            .map_or(0, |(_, chips)| *chips);
        if let Some(chips) = chips
            && (chips == 0
                || chips > rebuy.maximum_chips
                || chips > economy.available_coins().saturating_add(reserved_chips))
        {
            return Err(format!(
                "请输入 1 到 {} 的整数，且不能超过可用金币",
                rebuy.maximum_chips
            ));
        }
        let pending = PendingChoice {
            match_id: game.match_id,
            hand_number: rebuy.hand_number,
            player,
            chips,
            rejection_serial: model.rejection_serial(),
        };
        let previously_reserved = reserved_chips != 0;
        if let Some(chips) = chips {
            // 预留必须先落盘。总余额不扣预留，先上报最新余额再提交购买命令。
            economy.reserve_texas_rebuy(pending.match_id, chips)?;
            if !client.0.send(ClientCommand::PublishEconomy {
                coins: economy.coins(),
                avatar_frame: economy.equipped_frame(),
            }) {
                if !previously_reserved {
                    self.release(economy, pending.match_id)?;
                }
                return Err("余额上报失败，未提交补码".to_owned());
            }
        }
        if !client.0.send(game_command(TexasCommand::ChooseRebuy {
            hand_number: pending.hand_number,
            chips,
        })) {
            if chips.is_some() && !previously_reserved {
                self.release(economy, pending.match_id)?;
            }
            return Err("补码选择发送失败，请检查连接后重试".to_owned());
        }
        self.pending = Some(pending);
        Ok(())
    }

    pub(super) fn forget_inactive_choice(&mut self, active: Option<&PendingChoice>) {
        if self
            .pending
            .as_ref()
            .is_some_and(|pending| !active.is_some_and(|active| pending.retained_by(active)))
        {
            // 只丢弃输入状态，持久预留必须等待 Host 最终结论。
            self.pending = None;
        }
    }

    pub(super) fn release(
        &mut self,
        economy: &mut PlayerEconomy,
        match_id: MatchId,
    ) -> Result<(), String> {
        if let Err(error) = economy.cancel_texas_rebuy(match_id) {
            if !self.cancellations.contains(&match_id) {
                self.cancellations.push(match_id);
            }
            return Err(error);
        }
        Ok(())
    }
}
