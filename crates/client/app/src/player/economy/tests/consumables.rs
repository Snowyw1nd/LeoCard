use super::super::storage::{EconomyArchive, FORMAT_VERSION};
use super::super::{PropId, migration};
use leocard_protocol::{AvatarFrameId, MatchId, ProfileId};

#[test]
fn egg_throwers_cost_one_hundred_each_and_can_be_consumed_exactly_once() {
    let prop = PropId::AutoEggThrower;
    assert_eq!(prop.definition().price, 100);
    assert!(prop.is_consumable());
    let mut wallet = EconomyArchive::new(ProfileId([1; 32]));
    wallet.ledger.coins = 200;
    wallet.buy_consumable(prop).unwrap();
    wallet.buy_consumable(prop).unwrap();
    assert_eq!(wallet.ledger.coins, 0);
    assert_eq!(wallet.consumables[&prop], 2);
    assert!(wallet.buy_consumable(prop).is_err());
    assert_eq!(wallet.consumables[&prop], 2);
    wallet.consume_prop(prop).unwrap();
    assert_eq!(wallet.consumables[&prop], 1);
    wallet.consume_prop(prop).unwrap();
    assert!(wallet.consumables.is_empty());
    assert!(wallet.consume_prop(prop).is_err());
    assert_eq!(wallet.ledger.coins, 0);
    assert!(wallet.ledger.active_props.is_empty());
}

#[test]
fn quantity_limits_and_wrong_item_types_do_not_spend_coins() {
    let mut wallet = EconomyArchive::new(ProfileId([1; 32]));
    wallet.ledger.coins = 200;
    wallet.consumables.insert(PropId::AutoEggThrower, u32::MAX);
    assert!(wallet.buy_consumable(PropId::AutoEggThrower).is_err());
    assert!(wallet.buy_consumable(PropId::ObservationLens).is_err());
    assert!(wallet.consume_prop(PropId::ObservationLens).is_err());
    assert_eq!(wallet.ledger.coins, 200);
    assert_eq!(wallet.consumables[&PropId::AutoEggThrower], u32::MAX);
}

#[test]
fn migrating_v5_preserves_the_entire_wallet_and_starts_with_no_consumables() {
    let profile = ProfileId([1; 32]);
    let mut old = EconomyArchive::new(profile);
    old.ledger.coins = 1234;
    old.ledger
        .active_props
        .insert(PropId::ObservationLens, 98765);
    old.ledger
        .rewarded_achievements
        .insert("saved-reward".to_owned());
    old.ledger.settled_matches.insert(MatchId([2; 16]));
    old.owned_frames.insert(AvatarFrameId::Study);
    old.equipped_frame = AvatarFrameId::Study;
    old.exclusive_frame = Some(AvatarFrameId::Host);
    let payload = postcard::to_allocvec(&(
        &old.ledger,
        &old.owned_frames,
        old.equipped_frame,
        old.exclusive_frame,
    ))
    .unwrap();
    let migrated = migration::decode(5, &payload, profile, old.exclusive_frame).unwrap();
    assert_eq!(
        postcard::to_allocvec(&migrated.ledger).unwrap(),
        postcard::to_allocvec(&old.ledger).unwrap()
    );
    assert_eq!(migrated.owned_frames, old.owned_frames);
    assert_eq!(migrated.equipped_frame, old.equipped_frame);
    assert_eq!(migrated.exclusive_frame, old.exclusive_frame);
    assert!(migrated.consumables.is_empty());
}

#[test]
fn reloading_preserves_remaining_consumables_and_their_purchase_cost() {
    let mut wallet = EconomyArchive::new(ProfileId([1; 32]));
    wallet.ledger.coins = 300;
    wallet.buy_consumable(PropId::AutoEggThrower).unwrap();
    wallet.buy_consumable(PropId::AutoEggThrower).unwrap();
    wallet.consume_prop(PropId::AutoEggThrower).unwrap();
    let reloaded = migration::decode(
        FORMAT_VERSION,
        &postcard::to_allocvec(&wallet).unwrap(),
        wallet.ledger.profile_id,
        None,
    )
    .unwrap();
    assert_eq!(reloaded.ledger.coins, 100);
    assert_eq!(reloaded.consumables[&PropId::AutoEggThrower], 1);
}
