use leocard_protocol::{TexasPlayerView, TexasRebuyPurchase};

pub(super) struct TexasStanding<'a> {
    pub rank: usize,
    pub player: &'a TexasPlayerView,
    pub net_stack: i64,
}

pub(super) fn tournament_standings<'a>(
    players: &'a [TexasPlayerView],
    rebuys: &[TexasRebuyPurchase],
) -> Vec<TexasStanding<'a>> {
    let mut standings = players
        .iter()
        .map(|player| {
            let cost = rebuys
                .iter()
                .find(|purchase| purchase.player == player.player.id)
                .map_or(0, |purchase| purchase.chips);
            TexasStanding {
                rank: 0,
                player,
                // 所有人初始成本相同，比较盈亏只需扣除各自补码成本。
                net_stack: i64::from(player.stack) - i64::from(cost),
            }
        })
        .collect::<Vec<_>>();
    standings.sort_by(|left, right| {
        right
            .net_stack
            .cmp(&left.net_stack)
            .then_with(|| left.player.seat.0.cmp(&right.player.seat.0))
    });
    let mut previous = None;
    let mut rank = 0;
    for (index, standing) in standings.iter_mut().enumerate() {
        if previous != Some(standing.net_stack) {
            rank = index + 1;
        }
        standing.rank = rank;
        previous = Some(standing.net_stack);
    }
    standings
}
