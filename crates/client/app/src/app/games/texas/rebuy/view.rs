use super::super::TexasUiAction;
use super::state::{TexasRebuyCountdown, TexasRebuyInput, TexasRebuyUiState};
use crate::app::economy::TexasRebuyWallet;
use crate::app::presentation::{ACCENT, MUTED, TEXT, add_text};
use crate::app::runtime::{ClientResource, UiAssets};
use crate::app::shell::{
    CozyButtonVariant, UiAction, add_cozy_button_variant, add_cozy_disabled_button,
};
use bevy::prelude::*;
use bevy::ui_widgets::SelectAllOnFocus;
use leocard_protocol::TexasSnapshot;
use leocard_ui::{TextInput, UiComposer, spawn_node};

pub(in super::super) struct TexasRebuyContext<'a> {
    pub client: &'a ClientResource,
    pub wallet: &'a TexasRebuyWallet,
    pub state: &'a TexasRebuyUiState,
    pub available_coins: u32,
}

pub(in super::super) fn add_rebuy_controls(
    commands: &mut UiComposer,
    parent: Entity,
    game: &TexasSnapshot,
    context: &TexasRebuyContext<'_>,
    assets: &UiAssets,
) {
    let Some(rebuy) = &game.rebuy else {
        return;
    };
    let panel = commands.keyed_child(
        parent,
        "rebuy-window",
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: px(5),
            margin: UiRect::top(px(8)),
            ..default()
        },
    );
    let countdown = add_text(
        commands,
        panel,
        format!("补码确认剩余 {} 秒", context.state.seconds()),
        18.0,
        ACCENT,
        assets,
    );
    commands.entity(countdown).insert(TexasRebuyCountdown);
    add_text(
        commands,
        panel,
        "全体暂停 等待所有清空玩家确认后统一生效，等待期间不扣金币",
        14.0,
        TEXT,
        assets,
    );
    add_text(
        commands,
        panel,
        "任一拒绝、超时或断线即终场且不收补码费，补入筹码不计盈利",
        13.0,
        MUTED,
        assets,
    );
    let statuses = rebuy
        .choices
        .iter()
        .map(|choice| {
            let name = game
                .players
                .iter()
                .find(|view| view.player.id == choice.player)
                .map_or("玩家", |view| view.player.name.as_str());
            match choice.chips {
                Some(_) => format!("{name} 已确认"),
                None => format!("{name} 等待选择"),
            }
        })
        .collect::<Vec<_>>()
        .join("    ");
    add_text(commands, panel, statuses, 14.0, TEXT, assets);
    let model = context.client.0.model();
    let own_choice = model
        .you()
        .filter(|_| !model.is_spectating())
        .and_then(|you| rebuy.choices.iter().find(|choice| choice.player == you));
    let Some(choice) = own_choice else {
        add_text(
            commands,
            panel,
            "等待补码确认，暂不可准备下一手或返回大厅",
            14.0,
            MUTED,
            assets,
        );
        return;
    };
    if choice.chips.is_some() || context.wallet.submitted(game.match_id, rebuy.hand_number) {
        add_cozy_disabled_button(
            commands,
            panel,
            "已提交 等待全体确认",
            assets,
            px(240),
            36.0,
        );
        return;
    }
    add_text(
        commands,
        panel,
        format!(
            "每场仅首次清空可补码 1 金币 = 1 筹码 范围 1..{} 可用金币 {}",
            rebuy.maximum_chips, context.available_coins
        ),
        14.0,
        TEXT,
        assets,
    );
    let row = spawn_node(
        commands,
        panel,
        Node {
            align_items: AlignItems::Center,
            column_gap: px(10),
            ..default()
        },
        None,
    );
    let mut input = TextInput::new("texas.rebuy-chips", &context.state.amount);
    input.max_characters = 10;
    input.filter = |character| character.is_ascii_digit();
    input.font_size = 17.0;
    let editor = input.spawn(
        commands,
        row,
        Node {
            width: px(110),
            height: px(36),
            padding: UiRect::axes(px(10), px(3)),
            ..default()
        },
        assets.text_input_style(),
    );
    commands
        .entity(editor)
        .insert((TexasRebuyInput, SelectAllOnFocus));
    if context
        .state
        .chips(rebuy.maximum_chips, context.available_coins)
        .is_some()
        && context.state.seconds() > 0
    {
        add_cozy_button_variant(
            commands,
            row,
            "确认补码",
            UiAction::Texas(TexasUiAction::ConfirmRebuy),
            assets,
            px(130),
            36.0,
            CozyButtonVariant::Primary,
        );
    } else {
        add_cozy_disabled_button(commands, row, "确认补码", assets, px(130), 36.0);
    }
    add_cozy_button_variant(
        commands,
        row,
        "拒绝并终场",
        UiAction::Texas(TexasUiAction::DeclineRebuy),
        assets,
        px(140),
        36.0,
        CozyButtonVariant::Danger,
    );
}
