#![allow(
    clippy::float_cmp,
    reason = "test assertions compare exact values, not computed results"
)]

use std::time::Duration;

use bevy::prelude::*;
use bevy::time::{Fixed, Time};
use carcinisation_fps_core::pickup::PickupRules;
use carcinisation_net::{
    NetAttackId, NetHealth, NetPickup, NetPickupKind, NetPlayer, NetworkObjectId, PlayerId,
    PlayerNetState,
};
use carcinisation_server::systems::pickup_system;

pub fn test_app() -> App {
    let mut app = App::new();
    app.init_resource::<PickupRules>();
    app.init_resource::<carcinisation_server::systems::pickup::PickupEventBuffer>();
    // Init fixed time so pickup_system can read deterministic delta_secs().
    // We manually update delta before each update in tick_with.
    app.init_resource::<Time<Fixed>>();
    app.add_systems(Update, pickup_system);
    app
}

/// Run one app tick with a known fixed-time delta.
fn tick_with(app: &mut App, dt_secs: f32) {
    {
        let mut time = app.world_mut().resource_mut::<Time<Fixed>>();
        time.advance_by(Duration::from_secs_f32(dt_secs));
    }
    app.update();
}

/// Spawn a test player at `position` with `current` / 100 HP.
fn spawn_player(world: &mut World, position: Vec2, current: f32) -> Entity {
    world
        .spawn((
            NetPlayer {
                player_id: PlayerId(1),
                position,
                angle: 0.0,
                current_attack: NetAttackId::None,
                state: PlayerNetState::Alive,
                flame_active: false,
                avatar_palette_variant: None,
            },
            NetHealth {
                current,
                max: 100.0,
            },
        ))
        .id()
}

/// Spawn a test pickup.
fn spawn_pickup(
    world: &mut World,
    position: Vec2,
    kind: NetPickupKind,
    available: bool,
    respawn: Option<f32>,
    respawnable: bool,
) -> Entity {
    world
        .spawn(NetPickup {
            object_id: NetworkObjectId(1),
            position,
            kind,
            available,
            respawn_remaining: respawn,
            respawnable,
        })
        .id()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Health pickup heals 50 HP, clamped to max.
#[test]
fn heal_clamping() {
    let mut app = test_app();
    let player = spawn_player(app.world_mut(), Vec2::ZERO, 50.0);
    let pickup = spawn_pickup(
        app.world_mut(),
        Vec2::ZERO,
        NetPickupKind::Health,
        true,
        None,
        true,
    );

    tick_with(&mut app, 1.0 / 30.0);

    let health = app.world().get::<NetHealth>(player).unwrap();
    assert_eq!(health.current, 100.0, "healed 50 HP (clamped to 100)");

    // Second pickup: 90 → 100 (clamped).
    app.world_mut()
        .get_mut::<NetHealth>(player)
        .unwrap()
        .current = 90.0;
    {
        let mut pickup = app.world_mut().get_mut::<NetPickup>(pickup).unwrap();
        pickup.available = true;
        pickup.respawn_remaining = None;
    }
    tick_with(&mut app, 1.0 / 30.0);

    let health = app.world().get::<NetHealth>(player).unwrap();
    assert_eq!(health.current, 100.0, "clamped to max");
}

/// Unavailable pickups are ignored.
#[test]
fn unavailable_pickup_cannot_be_collected() {
    let mut app = test_app();
    let player = spawn_player(app.world_mut(), Vec2::ZERO, 50.0);
    let pickup = spawn_pickup(
        app.world_mut(),
        Vec2::ZERO,
        NetPickupKind::Health,
        false,
        Some(5.0),
        true,
    );

    tick_with(&mut app, 1.0 / 30.0);

    let health = app.world().get::<NetHealth>(player).unwrap();
    assert_eq!(health.current, 50.0, "health unchanged");

    let pickup = app.world().get::<NetPickup>(pickup).unwrap();
    assert!(!pickup.available, "still unavailable");
}

/// Respawn timer decrements and flips to available when it reaches zero.
#[test]
fn respawn_flips_available() {
    let mut app = test_app();
    let pickup_entity = spawn_pickup(
        app.world_mut(),
        Vec2::ZERO,
        NetPickupKind::Health,
        false,
        Some(0.1),
        true,
    );

    // Tick once with dt = 0.05 → remaining = 0.05.
    tick_with(&mut app, 0.05);
    let pickup = app.world().get::<NetPickup>(pickup_entity).unwrap();
    assert!(!pickup.available);
    assert_eq!(pickup.respawn_remaining, Some(0.05));

    // Tick again with dt = 0.1 → timer expires.
    tick_with(&mut app, 0.1);
    let pickup = app.world().get::<NetPickup>(pickup_entity).unwrap();
    assert!(pickup.available);
    assert_eq!(pickup.respawn_remaining, None);
}

/// Full-health players do not consume health pickups.
#[test]
fn full_health_does_not_consume() {
    let mut app = test_app();
    let player = spawn_player(app.world_mut(), Vec2::ZERO, 100.0);
    let pickup = spawn_pickup(
        app.world_mut(),
        Vec2::ZERO,
        NetPickupKind::Health,
        true,
        None,
        true,
    );

    tick_with(&mut app, 1.0 / 30.0);

    let health = app.world().get::<NetHealth>(player).unwrap();
    assert_eq!(health.current, 100.0, "no overheal");

    let pickup = app.world().get::<NetPickup>(pickup).unwrap();
    assert!(pickup.available, "pickup not consumed");
}

/// Dead players do not consume pickups even if they overlap.
#[test]
fn dead_player_does_not_consume() {
    let mut app = test_app();
    let player = spawn_player(app.world_mut(), Vec2::ZERO, 0.0);
    app.world_mut().get_mut::<NetPlayer>(player).unwrap().state = PlayerNetState::Dead;
    let pickup = spawn_pickup(
        app.world_mut(),
        Vec2::ZERO,
        NetPickupKind::Health,
        true,
        None,
        true,
    );

    tick_with(&mut app, 1.0 / 30.0);

    let health = app.world().get::<NetHealth>(player).unwrap();
    assert_eq!(health.current, 0.0, "dead player not healed");

    let pickup = app.world().get::<NetPickup>(pickup).unwrap();
    assert!(pickup.available, "pickup not consumed by dead player");
}

/// Unsupported pickup kinds (Ammo, Weapon) are safely ignored.
#[test]
fn unsupported_pickup_kind_is_noop() {
    for kind in &[NetPickupKind::Ammo, NetPickupKind::Weapon] {
        let mut app = test_app();
        let player = spawn_player(app.world_mut(), Vec2::ZERO, 50.0);
        let pickup = spawn_pickup(app.world_mut(), Vec2::ZERO, *kind, true, None, true);

        tick_with(&mut app, 1.0 / 30.0);

        let health = app.world().get::<NetHealth>(player).unwrap();
        assert_eq!(health.current, 50.0, "{kind:?} pickup must not heal");

        let pickup = app.world().get::<NetPickup>(pickup).unwrap();
        assert!(pickup.available, "{kind:?} pickup must not be consumed");
        assert_eq!(pickup.respawn_remaining, None);
    }
}

/// Unsupported pickups do not block another valid pickup in the same overlap set.
#[test]
fn unsupported_pickup_does_not_block_valid_pickup() {
    let mut app = test_app();
    let player = spawn_player(app.world_mut(), Vec2::ZERO, 50.0);
    let ammo_pickup = spawn_pickup(
        app.world_mut(),
        Vec2::ZERO,
        NetPickupKind::Ammo,
        true,
        None,
        true,
    );
    let health_pickup = spawn_pickup(
        app.world_mut(),
        Vec2::ZERO,
        NetPickupKind::Health,
        true,
        None,
        true,
    );

    tick_with(&mut app, 1.0 / 30.0);

    let health = app.world().get::<NetHealth>(player).unwrap();
    assert_eq!(health.current, 100.0, "health pickup still applied");

    let ammo = app.world().get::<NetPickup>(ammo_pickup).unwrap();
    assert!(ammo.available, "unsupported ammo pickup not consumed");

    let health = app.world().get::<NetPickup>(health_pickup).unwrap();
    assert!(!health.available, "health pickup consumed");
}

/// Non-respawnable pickups stay unavailable permanently after collection.
#[test]
fn non_respawnable_stays_unavailable() {
    let mut app = test_app();
    let player = spawn_player(app.world_mut(), Vec2::ZERO, 50.0);
    let pickup_entity = spawn_pickup(
        app.world_mut(),
        Vec2::ZERO,
        NetPickupKind::Health,
        true,
        None,
        false,
    );

    // Collect it.
    tick_with(&mut app, 1.0 / 30.0);

    let health = app.world().get::<NetHealth>(player).unwrap();
    assert_eq!(health.current, 100.0, "healed 50 HP");

    let pickup = app.world().get::<NetPickup>(pickup_entity).unwrap();
    assert!(!pickup.available, "pickup now unavailable");
    assert_eq!(pickup.respawn_remaining, None, "no respawn timer");

    // Advance time well beyond normal respawn — still unavailable.
    tick_with(&mut app, 60.0);
    let pickup = app.world().get::<NetPickup>(pickup_entity).unwrap();
    assert!(!pickup.available, "still unavailable after 60 seconds");
    assert_eq!(pickup.respawn_remaining, None);
}
