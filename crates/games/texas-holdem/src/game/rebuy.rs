use super::{TexasGameError, TexasGameState, TexasPhase};
use leocard_game_common::PlayerId;

impl TexasGameState {
    /// Refill every currently busted seat exactly once as an atomic batch after settlement.
    /// Each player may rebuy only once per match, for 1..=floor(starting_chips / 2) chips.
    /// The completed hand's final stacks remain the actual settlement snapshot.
    pub fn rebuy(&mut self, purchases: &[(PlayerId, u32)]) -> Result<(), TexasGameError> {
        if !self.rules.rebuy {
            return Err(TexasGameError::RebuyDisabled);
        }
        if !matches!(self.phase, TexasPhase::Complete(_)) {
            return Err(TexasGameError::HandStillInProgress);
        }
        let busted_count = self
            .players
            .iter()
            .filter(|player| player.stack == 0)
            .count();
        if purchases.is_empty() || purchases.len() != busted_count {
            return Err(TexasGameError::InvalidRebuy);
        }
        let maximum = u32::from(self.rules.starting_chips) / 2;
        let mut seen = vec![false; self.players.len()];
        for &(player, chips) in purchases {
            let state = self
                .players
                .get(player.index())
                .ok_or(TexasGameError::InvalidPlayer(player))?;
            if seen[player.index()] || state.stack != 0 || chips == 0 || chips > maximum {
                return Err(TexasGameError::InvalidRebuy);
            }
            if self.rebought[player.index()] != 0 {
                return Err(TexasGameError::RebuyAlreadyUsed(player));
            }
            seen[player.index()] = true;
        }
        for &(player, chips) in purchases {
            self.players[player.index()].stack = chips;
            self.rebought[player.index()] += chips;
        }
        Ok(())
    }

    /// Total extra chips bought by this seat in this match, including previous hands.
    ///
    /// # Panics
    /// Panics if `player` is not a valid player ID for this table.
    pub fn rebought_chips(&self, player: PlayerId) -> u32 {
        self.rebought[player.index()]
    }
}
