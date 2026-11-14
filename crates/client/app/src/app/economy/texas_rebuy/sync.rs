use super::TexasRebuyWallet;
use super::state::PendingChoice;
use crate::app::runtime::{ClientResource, PageErrorState, ServerNotification};
use bevy::prelude::*;
use leocard_client::{ClientPhaseRef, LocalPlayerProfile, NetworkState, PlayerEconomy};
use leocard_protocol::{GameSnapshot, ProfileId, ServerEvent};

pub(in super::super) fn sync_texas_rebuy_wallet(
    client: Option<Res<ClientResource>>,
    mut notifications: MessageReader<ServerNotification>,
    profile: Res<LocalPlayerProfile>,
    mut economy: ResMut<PlayerEconomy>,
    mut wallet: ResMut<TexasRebuyWallet>,
    mut page_error: ResMut<PageErrorState>,
    time: Res<Time>,
) {
    let profile_id = profile.identity.profile_id();
    let reservations = economy.pending_texas_rebuys();
    // 采集同帧所有结论后再入账，成功优先于取消，不使用旁观映射的 snapshot.you。
    for notification in notifications.read() {
        match &notification.event {
            ServerEvent::GameSnapshot(GameSnapshot::Texas(game)) => {
                wallet.observe_snapshot(notification.player, profile_id, game, &reservations);
            }
            ServerEvent::TexasRebuyResolved {
                match_id,
                profile_id: owner,
                chips,
            } => {
                wallet.observe_resolution(*owner, profile_id, *match_id, *chips);
            }
            _ => {}
        }
    }
    let connected = client
        .as_deref()
        .filter(|client| matches!(client.0.state(), NetworkState::Connected(_)));
    let unresolved = reservations.iter().any(|(match_id, _)| {
        !wallet.commits.iter().any(|(id, _)| id == match_id)
            && !wallet.cancellations.contains(match_id)
    });
    if connected.is_none() && unresolved {
        if !wallet.reservation_warning {
            page_error.error = Some("补码金额仍预留，请重连原房间确认结果".to_owned());
            wallet.reservation_warning = true;
        }
    } else {
        wallet.reservation_warning = false;
    }
    // 断线只清理内存输入，不释放持久预留，也不能让旧场次阻塞新场次。
    if connected.is_none() {
        wallet.forget_inactive_choice(None);
    }
    wallet.retry_after = (wallet.retry_after - time.delta_secs()).max(0.0);
    if wallet.retry_after > 0.0 {
        return;
    }
    let result = reconcile(connected, profile_id, &mut economy, &mut wallet);
    if let Err(error) = result {
        if wallet.last_error.as_deref() != Some(&error) {
            warn!("德州补码钱包同步失败：{error}");
            page_error.error = Some(format!("补码钱包同步失败：{error}，将自动重试"));
        }
        wallet.last_error = Some(error);
        wallet.retry_after = 0.5;
    } else {
        wallet.last_error = None;
    }
}

fn reconcile(
    connected: Option<&ClientResource>,
    profile_id: ProfileId,
    economy: &mut PlayerEconomy,
    wallet: &mut TexasRebuyWallet,
) -> Result<(), String> {
    while let Some(&(match_id, chips)) = wallet.commits.front() {
        economy.commit_texas_rebuy(match_id, chips)?;
        wallet.committed.insert(match_id);
        wallet.cancelled.remove(&match_id);
        wallet.commits.pop_front();
        wallet.cancellations.retain(|cancel| *cancel != match_id);
        if wallet
            .pending
            .as_ref()
            .is_some_and(|pending| pending.match_id == match_id)
        {
            wallet.pending = None;
        }
    }
    while let Some(&match_id) = wallet.cancellations.last() {
        // cancel 只删除 Reserved，绝不撤销 Committed 凭据。
        economy.cancel_texas_rebuy(match_id)?;
        wallet.cancellations.pop();
        if wallet
            .pending
            .as_ref()
            .is_some_and(|pending| pending.match_id == match_id)
        {
            wallet.pending = None;
        }
    }
    let active = connected.and_then(|client| {
        let model = client.0.model();
        let player = model.you()?;
        if model.is_spectating() {
            return None;
        }
        let ClientPhaseRef::Playing(GameSnapshot::Texas(game)) = model.phase() else {
            return None;
        };
        if !game
            .players
            .iter()
            .any(|view| view.player.id == player && view.player.profile_id == profile_id)
        {
            return None;
        }
        let rebuy = game.rebuy.as_ref()?;
        let choice = rebuy
            .choices
            .iter()
            .find(|choice| choice.player == player)?;
        Some(PendingChoice {
            match_id: game.match_id,
            hand_number: rebuy.hand_number,
            player,
            chips: choice.chips,
            rejection_serial: model.rejection_serial(),
        })
    });
    wallet.forget_inactive_choice(active.as_ref());
    // 重启或重连时 Host 仍在等待：恢复本人的预留，尚未提交则不扣费。
    if wallet.pending.is_none()
        && let Some(active) = active
        && let Some(chips) = active.chips
        && !wallet.committed.contains(&active.match_id)
        && !wallet.cancelled.contains(&active.match_id)
    {
        economy.reserve_texas_rebuy(active.match_id, chips)?;
        wallet.pending = Some(active);
    }
    Ok(())
}
