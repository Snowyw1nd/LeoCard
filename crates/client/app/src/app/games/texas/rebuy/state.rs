use bevy::prelude::*;
use leocard_protocol::MatchId;

#[derive(Resource, Default)]
pub(in super::super) struct TexasRebuyUiState {
    pub window: Option<(MatchId, u32)>,
    pub amount: String,
    pub remaining_ms: f32,
    pub observed_ms: u32,
}

impl TexasRebuyUiState {
    pub fn chips(&self, maximum: u32, available: u32) -> Option<u32> {
        let chips = self.amount.parse::<u32>().ok()?;
        (chips > 0 && chips <= maximum && chips <= available).then_some(chips)
    }

    pub fn seconds(&self) -> u32 {
        (self.remaining_ms / 1000.0).ceil() as u32
    }
}

#[derive(Component)]
pub(in super::super) struct TexasRebuyInput;

#[derive(Component)]
pub(in super::super) struct TexasRebuyCountdown;
