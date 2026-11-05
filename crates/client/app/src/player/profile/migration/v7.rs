//! 冻结 v7 根结构及游戏明细布局，新增根字段不得改变此版本的解码。

use super::super::{StoredGameProfiles, StoredPlayerProfile, StoredRatingProfile};
use leocard_blackjack::BlackjackProfileStats;
use leocard_doudizhu::DoudizhuProfileStats;
use leocard_mahjong::MahjongProfileStats;
use leocard_mc_guess::McGuessProfileStats;
use leocard_protocol::{MatchId, PlayerInteractionStats};
use leocard_qigui523::QiGui523ProfileStats;
use leocard_shengji::ShengjiProfileStats;
use leocard_texas_holdem::TexasProfileStats;
use leocard_uno::UnoProfileStats;
use serde::{Deserialize, Serialize};

pub(super) const V7_PROFILE_HEADER: &[u8] = b"LeoCard/profile/v7\0";

#[derive(Deserialize, Serialize)]
pub(super) struct V7PlayerProfile {
    secret_key: [u8; 32],
    rating: V7RatingProfile,
    interaction_stats: Option<PlayerInteractionStats>,
    games: V7GameProfiles,
}

#[derive(Deserialize, Serialize)]
struct V7RatingProfile {
    reference_points: i32,
    completed_games: u32,
    applied_matches: Vec<MatchId>,
}

#[derive(Deserialize, Serialize)]
struct V7GameProfiles {
    qigui523_stats: Option<QiGui523ProfileStats>,
    texas_stats: Option<TexasProfileStats>,
    shengji_stats: Option<ShengjiProfileStats>,
    uno_stats: Option<UnoProfileStats>,
    mahjong_stats: Option<MahjongProfileStats>,
    mc_guess_stats: Option<McGuessProfileStats>,
    blackjack_stats: Option<BlackjackProfileStats>,
    doudizhu_stats: Option<DoudizhuProfileStats>,
}

impl From<V7PlayerProfile> for StoredPlayerProfile {
    fn from(previous: V7PlayerProfile) -> Self {
        Self {
            secret_key: previous.secret_key,
            rating: StoredRatingProfile {
                reference_points: previous.rating.reference_points,
                completed_games: previous.rating.completed_games,
                applied_matches: previous.rating.applied_matches,
            },
            interaction_stats: previous.interaction_stats,
            games: StoredGameProfiles {
                qigui523_stats: previous.games.qigui523_stats,
                texas_stats: previous.games.texas_stats,
                shengji_stats: previous.games.shengji_stats,
                uno_stats: previous.games.uno_stats,
                mahjong_stats: previous.games.mahjong_stats,
                mc_guess_stats: previous.games.mc_guess_stats,
                blackjack_stats: previous.games.blackjack_stats,
                doudizhu_stats: previous.games.doudizhu_stats,
            },
            action_times: Default::default(),
        }
    }
}
