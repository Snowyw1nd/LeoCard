use super::TexasSession;
use crate::{ConnectionId, Delivery};
use leocard_game_common::PlayerId;
use leocard_protocol::ServerEvent;

impl TexasSession {
    pub(super) fn record_rebuy_receipts(&mut self, decisions: &[(PlayerId, Option<u32>)]) {
        let match_id = self
            .game
            .as_ref()
            .expect("rebuy requires a match")
            .match_id();
        for (id, chips) in decisions {
            let profile_id = self
                .room
                .players
                .iter()
                .find(|player| player.id == *id)
                .expect("rebuy participant belongs to the room")
                .profile_id;
            self.rebuy_receipts
                .entry(profile_id)
                .or_default()
                .push((match_id, *chips));
        }
    }

    /// 保留到房间关闭，回大厅后仍能按真实身份恢复未完成的钱包扣费或取消。
    pub(super) fn replay_rebuy_receipts(&self, connection: ConnectionId) -> Vec<Delivery> {
        let Some(player) = self.room.participants().find(|player| {
            player.connection == connection && player.connected && !player.left && !player.is_bot
        }) else {
            return Vec::new();
        };
        self.rebuy_receipts
            .get(&player.profile_id)
            .into_iter()
            .flatten()
            .map(|(match_id, chips)| {
                self.room.delivery(
                    connection,
                    None,
                    ServerEvent::TexasRebuyResolved {
                        match_id: *match_id,
                        profile_id: player.profile_id,
                        chips: *chips,
                    },
                )
            })
            .collect()
    }
}
