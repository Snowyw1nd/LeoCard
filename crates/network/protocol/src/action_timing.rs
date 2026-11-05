use crate::{GameKind, MatchId};
use leocard_game_common::PlayerId;
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

/// Cumulative human decision time; averages are weighted by completed operations.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct ActionTimeStats {
    pub total_millis: u64,
    pub completed_actions: u64,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct GameActionTimes {
    records: [ActionTimeStats; 8],
}

impl GameActionTimes {
    pub const fn get(&self, game: GameKind) -> ActionTimeStats {
        self.records[game_index(game)]
    }

    pub fn record(&mut self, game: GameKind, millis: u64) {
        let stats = &mut self.records[game_index(game)];
        if let (Some(total), Some(count)) = (
            stats.total_millis.checked_add(millis),
            stats.completed_actions.checked_add(1),
        ) {
            stats.total_millis = total;
            stats.completed_actions = count;
        }
    }
}

const fn game_index(game: GameKind) -> usize {
    match game {
        GameKind::QiGui523 => 0,
        GameKind::Texas => 1,
        GameKind::Shengji => 2,
        GameKind::Uno => 3,
        GameKind::Mahjong => 4,
        GameKind::McGuess => 5,
        GameKind::Blackjack => 6,
        GameKind::Doudizhu => 7,
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ActionClock {
    pub player: PlayerId,
    pub elapsed_millis: u64,
}

/// Authoritative elapsed time, also refreshed on transport heartbeats.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ActionTimingSnapshot {
    pub game: GameKind,
    pub match_id: MatchId,
    pub clocks: Vec<ActionClock>,
}
