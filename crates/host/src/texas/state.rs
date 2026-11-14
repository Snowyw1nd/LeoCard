use super::TexasAdapter;
use super::rebuy::RebuyWindow;
use crate::automation::BotDecision;
use crate::{AutomaticActionDelayState, RoomSession};
use leocard_protocol::{MatchId, PlayerReferenceChange, ProfileId};
use leocard_texas_holdem::{
    TexasAction, TexasBotObservation, TexasCard, TexasProfileStats, TexasRuleSet,
};
use std::collections::HashMap;

/// 复用 [`RoomSession`] 的德州扑克权威房间后端。
#[derive(Clone, Debug)]
pub struct TexasSession {
    pub(super) bot_decision: BotDecision<TexasBotObservation, TexasAction>,
    pub(super) room: RoomSession,
    pub(super) rules: TexasRuleSet,
    pub(super) shuffled_deck: Option<Vec<TexasCard>>,
    pub(super) game: Option<TexasAdapter>,
    pub(super) rebuy: Option<RebuyWindow>,
    pub(super) rebuy_receipts: HashMap<ProfileId, Vec<(MatchId, Option<u32>)>>,
    pub(super) match_profile_stats: Vec<TexasProfileStats>,
    pub(super) finished_reference_changes: Option<Vec<PlayerReferenceChange>>,
    pub(super) automatic_action_delay: Option<AutomaticActionDelayState>,
}
