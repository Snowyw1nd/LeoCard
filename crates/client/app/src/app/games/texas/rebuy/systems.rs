use super::state::{TexasRebuyCountdown, TexasRebuyInput, TexasRebuyUiState};
use crate::app::runtime::ClientResource;
use crate::app::shell::UiState;
use bevy::prelude::*;
use bevy::text::EditableText;
use leocard_client::PlayerEconomy;
use leocard_protocol::GameSnapshot;

pub(in super::super) fn sync_rebuy_clock(
    client: Option<Res<ClientResource>>,
    economy: Res<PlayerEconomy>,
    time: Res<Time>,
    mut state: ResMut<TexasRebuyUiState>,
    mut labels: Query<&mut Text, With<TexasRebuyCountdown>>,
    mut ui: ResMut<UiState>,
) {
    let rebuy = client.as_deref().and_then(|client| {
        let GameSnapshot::Texas(game) = client.0.model().game_snapshot()? else {
            return None;
        };
        Some((game.match_id, game.rebuy.as_ref()?))
    });
    let Some((match_id, rebuy)) = rebuy else {
        state.window = None;
        return;
    };
    let previous_seconds = state.seconds();
    let window = (match_id, rebuy.hand_number);
    if state.window != Some(window) {
        state.window = Some(window);
        state.amount = economy
            .pending_texas_rebuys()
            .into_iter()
            .find_map(|(id, chips)| (id == match_id).then_some(chips))
            .unwrap_or_else(|| {
                rebuy
                    .maximum_chips
                    .min(economy.available_texas_rebuy_coins(match_id))
                    .max(1)
            })
            .to_string();
        state.remaining_ms = rebuy.remaining_ms as f32;
        state.observed_ms = rebuy.remaining_ms;
    } else {
        state.remaining_ms = (state.remaining_ms - time.delta_secs() * 1000.0).max(0.0);
        if state.observed_ms != rebuy.remaining_ms {
            // 以 Host 剩余时间为上限，同 revision 心跳不能重置本地倒计时。
            state.remaining_ms = state.remaining_ms.min(rebuy.remaining_ms as f32);
            state.observed_ms = rebuy.remaining_ms;
        }
    }
    if previous_seconds > 0 && state.seconds() == 0 {
        ui.dirty = true;
    }
    let value = format!("补码确认剩余 {} 秒", state.seconds());
    for mut label in &mut labels {
        if label.0 != value {
            label.0.clone_from(&value);
        }
    }
}

pub(in super::super) fn sync_rebuy_input(
    inputs: Query<&EditableText, (With<TexasRebuyInput>, Changed<EditableText>)>,
    mut state: ResMut<TexasRebuyUiState>,
    mut ui: ResMut<UiState>,
) {
    for input in &inputs {
        let value = input.value().to_string();
        if state.amount != value {
            state.amount = value;
            ui.dirty = true;
        }
    }
}
