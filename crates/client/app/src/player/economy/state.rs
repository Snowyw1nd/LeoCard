use super::rebuy::TexasRebuyTransaction;
use super::rewards::login_day;
use super::storage::{EconomyArchive, unix_seconds};
use super::{PropId, ShopAvatarFrame};
use bevy::prelude::Resource;
use leocard_achievements::AchievementDefinition;
use leocard_protocol::{AvatarFrameId, MatchId, PlayerReferenceChange, ProfileId};
use std::mem;

#[derive(Resource)]
pub struct PlayerEconomy {
    archive: EconomyArchive,
    load_error: Option<String>,
    changes: Vec<i64>,
}

impl PlayerEconomy {
    pub fn load(profile_id: ProfileId, player_name: &str) -> Result<Self, String> {
        Ok(Self {
            archive: EconomyArchive::load(
                profile_id,
                ShopAvatarFrame::exclusive_for_name(player_name),
            )?,
            load_error: None,
            changes: Vec::new(),
        })
    }

    pub fn unavailable(profile_id: ProfileId, error: String) -> Self {
        Self {
            archive: EconomyArchive::new(profile_id),
            load_error: Some(error),
            changes: Vec::new(),
        }
    }

    pub fn error(&self) -> Option<&str> {
        self.load_error.as_deref()
    }

    pub fn coins(&self) -> u32 {
        self.archive.ledger.coins
    }

    /// 总余额减去尚未提交的补码预留。
    pub fn available_coins(&self) -> u32 {
        self.archive.available_coins()
    }

    /// 同场预留仍可用于恢复原购买，不应在重连界面中被当作余额不足。
    pub fn available_texas_rebuy_coins(&self, match_id: MatchId) -> u32 {
        let reserved = match self.archive.texas_rebuys.get(&match_id) {
            Some(TexasRebuyTransaction::Reserved(chips)) => *chips,
            _ => 0,
        };
        self.available_coins().saturating_add(reserved)
    }

    /// 仅读取当前已载入的钱包，不触发存档读取。
    pub fn pending_texas_rebuys(&self) -> Vec<(MatchId, u32)> {
        self.archive
            .texas_rebuys
            .iter()
            .filter_map(|(&match_id, transaction)| match transaction {
                TexasRebuyTransaction::Reserved(chips) => Some((match_id, *chips)),
                TexasRebuyTransaction::Committed(_) => None,
            })
            .collect()
    }

    /// 确认补码只持久化预留，等待 Host 的全员提交结果。
    pub fn reserve_texas_rebuy(&mut self, match_id: MatchId, chips: u32) -> Result<(), String> {
        if matches!(
            self.archive.texas_rebuys.get(&match_id),
            Some(TexasRebuyTransaction::Reserved(previous)) if *previous == chips
        ) {
            return Ok(());
        }
        self.transact(|archive| archive.reserve_texas_rebuy(match_id, chips))
    }

    /// 释放未提交的预留，不撤销已成功的购买凭据。
    pub fn cancel_texas_rebuy(&mut self, match_id: MatchId) -> Result<(), String> {
        if !matches!(
            self.archive.texas_rebuys.get(&match_id),
            Some(TexasRebuyTransaction::Reserved(_))
        ) {
            return Ok(());
        }
        self.transact(|archive| {
            archive.cancel_texas_rebuy(match_id);
            Ok(())
        })
    }

    /// 根据本人的 Host 成功记录扣款，首次返回 true，重复记录返回 false。
    /// 支持重连后无本地预留的恢复，已记录的金额不允许改变。
    pub fn commit_texas_rebuy(&mut self, match_id: MatchId, chips: u32) -> Result<bool, String> {
        if matches!(
            self.archive.texas_rebuys.get(&match_id),
            Some(TexasRebuyTransaction::Committed(previous)) if *previous == chips
        ) {
            return Ok(false);
        }
        self.transact(|archive| archive.commit_texas_rebuy(match_id, chips))
    }

    pub fn owns_frame(&self, frame: AvatarFrameId) -> bool {
        self.archive.owns_frame(frame)
    }

    pub fn is_exclusive_frame(&self, frame: AvatarFrameId) -> bool {
        self.archive.exclusive_frame == Some(frame)
    }

    pub fn equipped_frame(&self) -> AvatarFrameId {
        self.archive
            .frame_for_exclusive(self.archive.exclusive_frame)
    }

    pub fn frame_for_name(&self, player_name: &str) -> AvatarFrameId {
        if self.load_error.is_some() {
            return self.equipped_frame();
        }
        self.archive
            .frame_for_exclusive(ShopAvatarFrame::exclusive_for_name(player_name))
    }

    pub fn sync_exclusive_frame(&mut self, player_name: &str) -> Result<(), String> {
        let exclusive = ShopAvatarFrame::exclusive_for_name(player_name);
        if self.archive.exclusive_frame == exclusive {
            return Ok(());
        }
        self.transact(|archive| {
            archive.update_exclusive_frame(exclusive);
            Ok(())
        })
    }

    pub fn buy_frame(&mut self, frame: AvatarFrameId) -> Result<(), String> {
        self.transact(|archive| {
            if archive.can_use_frame(frame) {
                return Err("已可使用这个头像框".to_owned());
            }
            archive.spend(ShopAvatarFrame::definition(frame).price)?;
            archive.owned_frames.insert(frame);
            Ok(())
        })
    }

    pub fn equip_frame(&mut self, frame: AvatarFrameId) -> Result<bool, String> {
        if !self.archive.can_use_frame(frame) {
            return Err("尚未拥有这个头像框".to_owned());
        }
        if self.equipped_frame() == frame {
            return Ok(false);
        }
        self.transact(|archive| {
            archive.equipped_frame = frame;
            Ok(true)
        })
    }

    /// 只有成功保存的实际余额变化会产生提示，包括购买道具和开发者修改。
    pub fn take_coin_changes(&mut self) -> Vec<i64> {
        mem::take(&mut self.changes)
    }

    pub fn initialize_achievement_rewards(&mut self, earned: &[String]) -> Result<(), String> {
        if self.archive.ledger.rewards_initialized {
            return Ok(());
        }
        self.transact(|archive| {
            // 新钱包不补发启用金币奖励之前已经获得的成就。
            archive
                .ledger
                .rewarded_achievements
                .extend(earned.iter().cloned());
            archive.ledger.rewards_initialized = true;
            Ok(())
        })
    }

    pub fn claim_daily_login(&mut self) -> Result<bool, String> {
        let day = login_day(unix_seconds());
        if self
            .archive
            .ledger
            .last_login_day
            .is_some_and(|previous| previous >= day)
        {
            return Ok(false);
        }
        self.transact(|archive| Ok(archive.ledger.reward_daily(day)))
    }

    pub fn reward_achievement(
        &mut self,
        definition: &AchievementDefinition,
    ) -> Result<bool, String> {
        if self
            .archive
            .ledger
            .rewarded_achievements
            .contains(definition.id)
        {
            return Ok(false);
        }
        self.transact(|archive| Ok(archive.ledger.reward_achievement(definition)))
    }

    pub fn settle_match(
        &mut self,
        match_id: MatchId,
        changes: &[PlayerReferenceChange],
    ) -> Result<bool, String> {
        if self.archive.ledger.settled_matches.contains(&match_id)
            || !changes
                .iter()
                .any(|change| change.profile_id == self.archive.ledger.profile_id)
        {
            return Ok(false);
        }
        self.transact(|archive| Ok(archive.settle_match(match_id, changes)))
    }

    #[cfg(feature = "developer")]
    pub fn set_developer_coins(&mut self, coins: u32) -> Result<(), String> {
        self.transact(|archive| {
            if coins < archive.reserved_coins() {
                return Err("余额不能低于已预留金币".to_owned());
            }
            archive.ledger.coins = coins;
            Ok(())
        })
    }

    pub fn prop_count(&self, prop: PropId) -> u32 {
        if self.load_error.is_some() {
            return 0;
        }
        self.archive.consumables.get(&prop).copied().unwrap_or(0)
    }

    pub fn consume_prop(&mut self, prop: PropId) -> Result<(), String> {
        self.transact(|archive| archive.consume_prop(prop))
    }

    pub fn remaining_seconds(&self, prop: PropId) -> u64 {
        if self.load_error.is_some() || prop.is_consumable() {
            return 0;
        }
        self.archive
            .ledger
            .active_props
            .get(&prop.canonical())
            .copied()
            .unwrap_or(0)
            .saturating_sub(unix_seconds())
    }

    pub fn active(&self, prop: PropId) -> bool {
        self.remaining_seconds(prop) > 0
    }

    pub fn buy_prop(&mut self, prop: PropId) -> Result<(), String> {
        if prop.is_consumable() {
            return self.transact(|archive| archive.buy_consumable(prop));
        }
        let prop = prop.canonical();
        if self.active(prop) {
            return Err("道具已生效，到期后可再次购买".to_owned());
        }
        let expires = unix_seconds()
            .checked_add(prop.definition().duration_seconds)
            .ok_or("道具到期时间超出范围")?;
        self.purchase_until(prop, expires)
    }

    pub fn extend_prop(&mut self, prop: PropId) -> Result<(), String> {
        let prop = prop.canonical();
        if !self.active(prop) {
            return Err("道具已到期，请重新购买".to_owned());
        }
        let expires = self.archive.ledger.active_props[&prop]
            .checked_add(prop.definition().duration_seconds)
            .ok_or("道具到期时间超出范围")?;
        self.purchase_until(prop, expires)
    }

    fn purchase_until(&mut self, prop: PropId, expires: u64) -> Result<(), String> {
        self.transact(|archive| {
            archive.spend(prop.definition().price)?;
            archive
                .ledger
                .active_props
                .insert(prop.canonical(), expires);
            Ok(())
        })
    }

    fn transact<T>(
        &mut self,
        change: impl FnOnce(&mut EconomyArchive) -> Result<T, String>,
    ) -> Result<T, String> {
        if let Some(error) = &self.load_error {
            return Err(error.clone());
        }
        let mut updated = self.archive.clone();
        let result = change(&mut updated)?;
        updated.save()?;
        let delta = i64::from(updated.ledger.coins) - i64::from(self.archive.ledger.coins);
        self.archive = updated;
        if delta != 0 {
            self.changes.push(delta);
        }
        Ok(result)
    }
}
