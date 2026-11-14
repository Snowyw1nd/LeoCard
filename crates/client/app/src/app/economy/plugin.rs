use super::texas_rebuy::sync_texas_rebuy_wallet;
use super::{TexasRebuyWallet, notifications::*, rewards::*, sync::*};
use crate::app::achievements::AchievementUpdateSet;
use crate::app::runtime::ClientUpdateSet;
use bevy::prelude::*;

pub(crate) struct EconomyPlugin;

impl Plugin for EconomyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PendingRewards>()
            .init_resource::<TexasRebuyWallet>()
            .add_systems(PostStartup, setup_rewards)
            .add_systems(
                Update,
                (
                    sync_texas_rebuy_wallet,
                    sync_finished_profile,
                    process_rewards,
                    sync_economy,
                    publish_economy,
                )
                    .chain()
                    .after(AchievementUpdateSet)
                    .in_set(ClientUpdateSet::Sync),
            )
            .add_systems(
                Update,
                queue_coin_notifications.in_set(ClientUpdateSet::Sync),
            );
    }
}
