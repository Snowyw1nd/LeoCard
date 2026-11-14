//! 德州扑克按钮动作到本地状态或网络命令的转换。

use super::betting::raise_sizing;
use super::rebuy::TexasRebuyUiState;
use super::{TexasHandGuideState, TexasUiState};
use crate::app::economy::TexasRebuyWallet;
use crate::app::runtime::{ClientResource, PageErrorState};
use crate::app::shell::{
    DomainUiAction, PressedUiAction, UiAction, UiActionHandler, dispatch_domain_actions,
    game_command, send_game_command,
};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use leocard_client::{PlayerEconomy, PropId};
use leocard_protocol::{GameSnapshot, TexasCommand};
use leocard_texas_holdem::{TexasAction, TexasRuleSet};

#[derive(Clone)]
pub(crate) enum TexasUiAction {
    UpdateRules(TexasRuleSet),
    SetRaiseTo(u32),
    BeginRaise,
    CancelRaise,
    Raise,
    Act(TexasAction),
    ChooseRunout(bool),
    ConfirmRebuy,
    DeclineRebuy,
    ToggleHandGuide,
    CloseHandGuide,
    ToggleSpectatorDrawer,
    ToggleWinRates,
}

impl DomainUiAction for TexasUiAction {
    fn rebuilds_ui(&self) -> bool {
        !matches!(
            self,
            Self::ToggleHandGuide | Self::CloseHandGuide | Self::SetRaiseTo(_)
        )
    }

    fn extract(action: &UiAction) -> Option<&Self> {
        let UiAction::Texas(action) = action else {
            return None;
        };
        Some(action)
    }
}

#[derive(SystemParam)]
pub(crate) struct TexasActionContext<'w> {
    client: Option<ResMut<'w, ClientResource>>,
    ui: ResMut<'w, TexasUiState>,
    guide: ResMut<'w, TexasHandGuideState>,
    page_error: ResMut<'w, PageErrorState>,
    economy: ResMut<'w, PlayerEconomy>,
    rebuy_wallet: ResMut<'w, TexasRebuyWallet>,
    rebuy_ui: Res<'w, TexasRebuyUiState>,
}

pub(super) fn dispatch_texas_actions(
    mut actions: MessageReader<PressedUiAction>,
    mut context: TexasActionContext,
) {
    dispatch_domain_actions::<TexasUiAction, _>(&mut actions, &mut context);
}

impl UiActionHandler<TexasActionContext<'_>> for TexasUiAction {
    fn handle(&self, context: &mut TexasActionContext<'_>) {
        match self {
            Self::ToggleSpectatorDrawer => {
                context.ui.spectator.drawer_open = !context.ui.spectator.drawer_open
            }
            Self::ToggleWinRates => {
                if !context.economy.active(PropId::ObservationLens) {
                    return;
                }
                let preferences = &mut context.ui.spectator.preferences;
                preferences.show_win_rates = !preferences.show_win_rates;
                if let Err(error) = preferences.save() {
                    preferences.show_win_rates = !preferences.show_win_rates;
                    context.page_error.error = Some(error);
                }
            }
            Self::ConfirmRebuy | Self::DeclineRebuy => {
                let Some(client) = context.client.as_deref_mut() else {
                    return;
                };
                let chips = if matches!(self, Self::ConfirmRebuy) {
                    let Some(GameSnapshot::Texas(game)) = client.0.model().game_snapshot() else {
                        return;
                    };
                    let Some(rebuy) = &game.rebuy else {
                        return;
                    };
                    let Some(chips) = context.rebuy_ui.chips(
                        rebuy.maximum_chips,
                        context.economy.available_texas_rebuy_coins(game.match_id),
                    ) else {
                        context.page_error.error =
                            Some("请输入范围内的整数，且不能超过可用金币".to_owned());
                        return;
                    };
                    if context.rebuy_ui.seconds() == 0 {
                        return;
                    }
                    Some(chips)
                } else {
                    None
                };
                if let Err(error) = context
                    .rebuy_wallet
                    .choose(client, &mut context.economy, chips)
                {
                    context.page_error.error = Some(error);
                }
            }
            Self::ToggleHandGuide => context.guide.toggle(),
            Self::CloseHandGuide => context.guide.close(),
            TexasUiAction::UpdateRules(rules) => {
                send_game_command(
                    &mut context.client,
                    TexasCommand::UpdateRules { rules: *rules },
                );
            }
            TexasUiAction::ChooseRunout(twice) => {
                send_game_command(
                    &mut context.client,
                    TexasCommand::ChooseRunout { twice: *twice },
                );
            }
            TexasUiAction::SetRaiseTo(target) => {
                context.ui.raise_to = *target;
                context.ui.raise_input_valid = true;
            }
            TexasUiAction::BeginRaise => context.ui.raise_editor_open = true,
            TexasUiAction::CancelRaise => context.ui.raise_editor_open = false,
            TexasUiAction::Raise => {
                if context.ui.raise_pending.is_some() {
                    return;
                }
                let Some(sizing) = context.client.as_deref().and_then(|client| {
                    match client.0.model().game_snapshot()? {
                        GameSnapshot::Texas(game) => raise_sizing(game),
                        _ => None,
                    }
                }) else {
                    return;
                };
                if !context.ui.raise_input_valid {
                    return;
                }
                let Some(target) = sizing.normalize(context.ui.raise_to) else {
                    return;
                };
                let Some(client) = context.client.as_deref_mut() else {
                    return;
                };
                let pending = (
                    client.0.model().latest_revision(),
                    client.0.model().rejection_serial(),
                );
                if client.0.send(game_command(TexasCommand::Act {
                    action: TexasAction::RaiseTo(target),
                })) {
                    context.ui.raise_pending = Some(pending);
                }
            }
            TexasUiAction::Act(action) => {
                send_game_command(&mut context.client, TexasCommand::Act { action: *action });
            }
        }
    }
}
