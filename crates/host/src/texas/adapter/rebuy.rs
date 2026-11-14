use super::{AdapterError, TexasAdapter};
use leocard_game_common::PlayerId;

impl TexasAdapter {
    pub(in super::super) fn rebuy(
        &mut self,
        purchases: &[(PlayerId, u32)],
    ) -> Result<(), AdapterError> {
        let core = purchases
            .iter()
            .map(|(id, chips)| self.core_player(*id).map(|id| (id, *chips)))
            .collect::<Result<Vec<_>, _>>()?;
        self.game.rebuy(&core).map_err(AdapterError::Game)?;
        // 本手盈利不包括外部购入筹码，下一手会重新记录真正的手前筹码。
        for (id, chips) in core {
            self.hand_starting_stacks[id.index()] += chips;
        }
        Ok(())
    }
}
