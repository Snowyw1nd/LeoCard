use super::TexasRebuyWallet;
use leocard_game_common::PlayerId;
use leocard_protocol::{MatchId, ProfileId, TexasRebuyPurchase, TexasSnapshot};

impl TexasRebuyWallet {
    pub(super) fn observe_snapshot(
        &mut self,
        player: Option<PlayerId>,
        profile_id: ProfileId,
        game: &TexasSnapshot,
        reservations: &[(MatchId, u32)],
    ) {
        // snapshot.you 可能是旁观目标，必须核对连接身份及真实档案。
        let Some(player) = own_player(
            player,
            profile_id,
            game.players
                .iter()
                .map(|view| (view.player.id, view.player.profile_id)),
        ) else {
            return;
        };
        let reserved = reservations.iter().any(|(id, _)| *id == game.match_id);
        if let Some(chips) = snapshot_outcome(player, &game.rebuys, game.rebuy.is_some(), reserved)
        {
            self.observe_outcome(game.match_id, chips);
        }
    }

    pub(super) fn observe_resolution(
        &mut self,
        owner: ProfileId,
        local_profile: ProfileId,
        match_id: MatchId,
        chips: Option<u32>,
    ) {
        if owner == local_profile {
            self.observe_outcome(match_id, chips);
        }
    }

    fn observe_outcome(&mut self, match_id: MatchId, chips: Option<u32>) {
        if self.committed.contains(&match_id) {
            return;
        }
        if let Some(chips) = chips {
            self.cancelled.remove(&match_id);
            if !self.commits.iter().any(|(id, _)| *id == match_id) {
                self.commits.push_back((match_id, chips));
            }
            self.cancellations.retain(|id| *id != match_id);
        } else if !self.commits.iter().any(|(id, _)| *id == match_id) {
            self.cancelled.insert(match_id);
            if !self.cancellations.contains(&match_id) {
                self.cancellations.push(match_id);
            }
        }
    }
}

fn own_player(
    player: Option<PlayerId>,
    profile_id: ProfileId,
    mut participants: impl Iterator<Item = (PlayerId, ProfileId)>,
) -> Option<PlayerId> {
    player.filter(|player| participants.any(|entry| entry == (*player, profile_id)))
}

fn snapshot_outcome(
    player: PlayerId,
    purchases: &[TexasRebuyPurchase],
    awaiting_rebuy: bool,
    reserved: bool,
) -> Option<Option<u32>> {
    if let Some(purchase) = purchases.iter().find(|purchase| purchase.player == player) {
        Some(Some(purchase.chips))
    } else if reserved && !awaiting_rebuy {
        // 同场真实玩家的窗口已结束，且没有购买记录，才是权威取消。
        Some(None)
    } else {
        None
    }
}
