use super::AutoEggThrower;
use super::state::EggBurst;
use super::systems::tick_auto_egg_thrower;
use crate::app::runtime::{ClientResource, PageErrorState};
use bevy::prelude::*;
use leocard_client::{
    ClientModel, LocalPlayerConnection, LocalPlayerProfile, PlayerIdentity, PlayerRatingProfile,
    TcpGameClient,
};
use leocard_game_common::PlayerId;
use leocard_protocol::{PlayerInteractionKind, ProfileId, RoomId};
use leocard_qigui523::QiGuiRuleSet;
use std::net::TcpListener;
use std::thread;
use std::time::{Duration, Instant};

fn burst() -> EggBurst {
    EggBurst {
        room: RoomId(1),
        source: PlayerId(0),
        target: PlayerId(1),
        target_profile: ProfileId([2; 32]),
        started_at: Duration::from_secs(20),
        sent: 0,
    }
}

#[test]
fn eggs_are_spaced_at_one_hundred_milliseconds_and_stop_at_one_hundred() {
    let burst = burst();
    for (millis, expected) in [
        (0, 0),
        (99, 0),
        (100, 1),
        (9999, 99),
        (10000, 100),
        (60000, 100),
    ] {
        assert_eq!(
            burst.due(burst.started_at + Duration::from_millis(millis)),
            expected
        );
    }
    assert_eq!(burst.due(Duration::ZERO), 0);
}

#[test]
fn irregular_frames_catch_up_without_duplicate_or_extra_eggs() {
    let mut burst = burst();
    let mut emitted = 0;
    for millis in [0, 50, 100, 100, 370, 990, 2100, 4000, 9999, 10000, 11000] {
        let due = burst.due(burst.started_at + Duration::from_millis(millis));
        while burst.sent < due {
            emitted += 1;
            burst.sent += 1;
        }
        assert_eq!(burst.finished(), millis >= 10000);
    }
    assert_eq!(emitted, 100);
}

fn wait_for(client: &mut TcpGameClient, ready: impl Fn(&ClientModel) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        client.poll();
        if ready(client.model()) {
            return;
        }
        thread::sleep(Duration::from_millis(5));
    }
    panic!("timed out waiting for local room: {:?}", client.state());
}

#[test]
fn a_real_tcp_target_receives_exactly_one_hundred_host_validated_eggs() {
    let port = TcpListener::bind(("127.0.0.1", 0))
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let mut host = TcpGameClient::host_with_profile(
        port,
        QiGuiRuleSet::default().into(),
        LocalPlayerConnection::from_profile(
            "砸蛋器房主",
            None,
            &LocalPlayerProfile {
                identity: PlayerIdentity::generate().unwrap(),
                rating: PlayerRatingProfile {
                    reference_points: 0,
                    completed_games: 0,
                    applied_matches: default(),
                    last_change: None,
                },
                game_profiles: default(),
            },
            default(),
            default(),
        ),
    )
    .unwrap();
    wait_for(&mut host, |model| model.lobby().is_some());
    let mut guest = TcpGameClient::join("目标", &format!("127.0.0.1:{port}")).unwrap();
    wait_for(&mut guest, |model| model.lobby().is_some());
    let target = guest.model().you().unwrap();
    wait_for(&mut host, |model| model.player(target).is_some());
    let active = EggBurst {
        room: host.model().room_id(),
        source: host.model().you().unwrap(),
        target,
        target_profile: host.model().player(target).unwrap().profile_id,
        started_at: Duration::ZERO,
        sent: 0,
    };
    let mut app = App::new();
    app.insert_resource(Time::<Real>::default())
        .insert_resource(ClientResource(host))
        .insert_resource(PageErrorState::default())
        .insert_resource(AutoEggThrower {
            active: Some(active),
        })
        .add_systems(Update, tick_auto_egg_thrower);
    let mut received = 0;
    for expected in 1..=100 {
        app.world_mut()
            .resource_mut::<Time<Real>>()
            .advance_by(Duration::from_millis(100));
        app.update();
        let deadline = Instant::now() + Duration::from_secs(5);
        while received < expected && Instant::now() < deadline {
            guest.poll();
            for interaction in guest.take_player_interactions() {
                assert_eq!(interaction.target, target);
                assert_eq!(interaction.kind, PlayerInteractionKind::Egg);
                received += 1;
            }
            thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(received, expected);
    }
    assert!(app.world().resource::<AutoEggThrower>().active.is_none());
    assert!(app.world().resource::<PageErrorState>().error.is_none());
    assert!(guest.model().last_rejection().is_none());
}

#[test]
fn leaving_the_room_cancels_the_burst_instead_of_leaking_it_to_another_session() {
    let mut app = App::new();
    app.insert_resource(Time::<Real>::default())
        .insert_resource(PageErrorState::default())
        .insert_resource(AutoEggThrower {
            active: Some(burst()),
        })
        .add_systems(Update, tick_auto_egg_thrower);
    app.update();
    assert!(app.world().resource::<AutoEggThrower>().active.is_none());
}
