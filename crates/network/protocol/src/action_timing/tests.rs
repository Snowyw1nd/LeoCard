use super::{ActionTimeStats, GameActionTimes};
use crate::GameKind;

#[test]
fn all_games_keep_independent_weighted_totals_and_round_trip_without_loss() {
    let mut times = GameActionTimes::default();
    let games = [
        GameKind::QiGui523,
        GameKind::Texas,
        GameKind::Shengji,
        GameKind::Uno,
        GameKind::Mahjong,
        GameKind::McGuess,
        GameKind::Blackjack,
        GameKind::Doudizhu,
    ];
    for (index, game) in games.into_iter().enumerate() {
        times.record(game, index as u64 * 1000);
        times.record(game, 500);
    }
    for (index, game) in games.into_iter().enumerate() {
        assert_eq!(
            times.get(game),
            ActionTimeStats {
                total_millis: index as u64 * 1000 + 500,
                completed_actions: 2,
            }
        );
    }
    let decoded: GameActionTimes =
        postcard::from_bytes(&postcard::to_allocvec(&times).unwrap()).unwrap();
    assert_eq!(decoded, times);
}

#[test]
fn overflowing_totals_do_not_increment_the_sample_count() {
    let mut times = GameActionTimes::default();
    times.record(GameKind::Texas, u64::MAX);
    times.record(GameKind::Texas, 1);
    assert_eq!(
        times.get(GameKind::Texas),
        ActionTimeStats {
            total_millis: u64::MAX,
            completed_actions: 1
        }
    );
}
