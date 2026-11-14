use super::storage::EconomyArchive;
use leocard_protocol::MatchId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
pub(super) enum TexasRebuyTransaction {
    Reserved(u32),
    Committed(u32),
}

impl EconomyArchive {
    pub(super) fn reserved_coins(&self) -> u32 {
        self.texas_rebuys
            .values()
            .fold(0_u32, |total, transaction| {
                total.saturating_add(match transaction {
                    TexasRebuyTransaction::Reserved(chips) => *chips,
                    TexasRebuyTransaction::Committed(_) => 0,
                })
            })
    }

    pub(super) fn available_coins(&self) -> u32 {
        self.ledger.coins.saturating_sub(self.reserved_coins())
    }

    /// 所有主动消费经过钱包层，不让奖励账本依赖补码交易。
    pub(super) fn spend(&mut self, coins: u32) -> Result<(), String> {
        if coins > self.available_coins() {
            return Err("可用金币不足".to_owned());
        }
        self.ledger.spend(coins)
    }

    pub(super) fn reserve_texas_rebuy(
        &mut self,
        match_id: MatchId,
        chips: u32,
    ) -> Result<(), String> {
        match self.texas_rebuys.get(&match_id) {
            Some(TexasRebuyTransaction::Reserved(previous)) if *previous == chips => {
                return Ok(());
            }
            Some(TexasRebuyTransaction::Reserved(_)) => {
                return Err("本场补码金额不一致".to_owned());
            }
            Some(TexasRebuyTransaction::Committed(_)) => {
                return Err("每场只能补码一次".to_owned());
            }
            None => {}
        }
        if chips == 0 {
            return Err("补码数量必须大于零".to_owned());
        }
        if chips > self.available_coins() {
            return Err("可用金币不足".to_owned());
        }
        self.texas_rebuys
            .insert(match_id, TexasRebuyTransaction::Reserved(chips));
        Ok(())
    }

    pub(super) fn cancel_texas_rebuy(&mut self, match_id: MatchId) {
        // 成功凭据不能被取消或清理，否则旧快照可能再次扣款。
        if matches!(
            self.texas_rebuys.get(&match_id),
            Some(TexasRebuyTransaction::Reserved(_))
        ) {
            self.texas_rebuys.remove(&match_id);
        }
    }

    pub(super) fn commit_texas_rebuy(
        &mut self,
        match_id: MatchId,
        chips: u32,
    ) -> Result<bool, String> {
        let reserved = match self.texas_rebuys.get(&match_id) {
            Some(TexasRebuyTransaction::Committed(previous)) if *previous == chips => {
                return Ok(false);
            }
            Some(TexasRebuyTransaction::Reserved(previous)) if *previous == chips => *previous,
            Some(_) => return Err("本场补码金额不一致".to_owned()),
            None => 0,
        };
        if chips == 0 {
            return Err("补码数量必须大于零".to_owned());
        }
        // 无预留也可从 Host 成功记录恢复，但不能占用其他场次的预留。
        if chips > self.available_coins().saturating_add(reserved) {
            return Err("可用金币不足".to_owned());
        }
        self.ledger.spend(chips)?;
        self.texas_rebuys
            .insert(match_id, TexasRebuyTransaction::Committed(chips));
        Ok(true)
    }
}
