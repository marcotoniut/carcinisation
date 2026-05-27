//! ECS-level map reset integration tests.
//!
//! Tests cover enemy despawn/respawn, player preservation, projectile cleanup,
//! dead-player revival, and per-player resource clearing on reset.
//!
//! Admin socket tests are in `admin_socket.rs`.
#![allow(clippy::float_cmp)]

mod common;

use bevy::prelude::*;
use carcinisation_net::{
    NetHealth, NetPickup, NetPlayer, NetProjectileType, NetworkObjectId, Owner, PlayerId,
    PlayerNetState,
};
use carcinisation_server::systems::reset::MapResetRequested;
use carcinisation_server::systems::{
    FireCooldownMap, FlameActiveTracker, NetEnemy, NetProjectile, ProjectileTtl, RespawnTimer,
};
use common::reset::{
    build_reset_server, count, one_enemy, spawn_player, tick_server_n, two_pickups,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn pickup_object_ids(app: &mut App) -> Vec<NetworkObjectId> {
    app.world_mut()
        .query::<&NetPickup>()
        .iter(app.world())
        .map(|pickup| pickup.object_id)
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn reset_despawns_enemies_and_respawns_them() {
    let mut server = build_reset_server(one_enemy(), None);
    server.update();
    assert_eq!(count::<NetEnemy>(&mut server), 1);

    let enemy_entity = server
        .world_mut()
        .query_filtered::<Entity, With<NetEnemy>>()
        .iter(server.world())
        .next()
        .unwrap();
    server
        .world_mut()
        .entity_mut(enemy_entity)
        .get_mut::<NetHealth>()
        .unwrap()
        .current = 0.0;

    tick_server_n(&mut server, 300);

    server.world_mut().resource_mut::<MapResetRequested>().0 = true;
    tick_server_n(&mut server, 30);

    assert_eq!(count::<NetEnemy>(&mut server), 1);
}

#[test]
fn reset_despawns_pickups_and_respawns_without_duplicate_ids() {
    let mut server = build_reset_server(two_pickups(), None);
    server.update();

    let ids_before = pickup_object_ids(&mut server);
    assert_eq!(ids_before.len(), 2);
    let unique_before = ids_before
        .iter()
        .copied()
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(unique_before.len(), ids_before.len());

    server.world_mut().resource_mut::<MapResetRequested>().0 = true;
    tick_server_n(&mut server, 30);

    let ids_after = pickup_object_ids(&mut server);
    assert_eq!(ids_after.len(), 2, "reset must not duplicate pickups");
    let unique_after = ids_after
        .iter()
        .copied()
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(
        unique_after.len(),
        ids_after.len(),
        "reset must not leave duplicate pickup object IDs"
    );
}

#[test]
fn reset_preserves_players_and_restores_health() {
    let mut server = build_reset_server(vec![], None);
    server.update();

    let player_entity = spawn_player(&mut server, 1, 5.0, 5.0);

    server
        .world_mut()
        .entity_mut(player_entity)
        .get_mut::<NetHealth>()
        .unwrap()
        .current = 10.0;

    server.world_mut().resource_mut::<MapResetRequested>().0 = true;
    tick_server_n(&mut server, 30);

    assert_eq!(count::<NetPlayer>(&mut server), 1);

    let health = server
        .world()
        .entity(player_entity)
        .get::<NetHealth>()
        .unwrap();
    assert_eq!(health.current, health.max);

    let player = server
        .world()
        .entity(player_entity)
        .get::<NetPlayer>()
        .unwrap();
    assert_eq!(player.state, PlayerNetState::Alive);
}

#[test]
fn reset_despawns_projectiles() {
    let mut server = build_reset_server(vec![], None);
    server.update();

    server.world_mut().spawn((
        carcinisation_net::NetProjectile {
            object_id: NetworkObjectId(99),
            position: Vec2::new(3.0, 3.0),
            angle: 0.0,
            owner: Owner(PlayerId(0)),
            damage: 10.0,
            projectile_type: NetProjectileType::BloodShot,
        },
        ProjectileTtl(5.0),
    ));

    assert_eq!(count::<NetProjectile>(&mut server), 1);

    server.world_mut().resource_mut::<MapResetRequested>().0 = true;
    tick_server_n(&mut server, 30);

    assert_eq!(count::<NetProjectile>(&mut server), 0);
}

#[test]
fn reset_despawns_pending_projectiles() {
    let mut server = build_reset_server(one_enemy(), None);
    server.update();

    let enemy_entity = server
        .world_mut()
        .query_filtered::<Entity, With<NetEnemy>>()
        .iter(server.world())
        .next()
        .unwrap();

    server.world_mut().spawn(
        carcinisation_server::systems::enemy_attack::PendingProjectile {
            timer: 1.0,
            source_entity: enemy_entity,
            position: Vec2::new(3.5, 1.5),
            angle: 0.0,
            damage: 10.0,
            object_id: NetworkObjectId(500),
            projectile_type: carcinisation_net::NetProjectileType::BloodShot,
        },
    );

    assert_eq!(
        count::<carcinisation_server::systems::enemy_attack::PendingProjectile>(&mut server),
        1
    );

    server.world_mut().resource_mut::<MapResetRequested>().0 = true;
    tick_server_n(&mut server, 30);

    assert_eq!(
        count::<carcinisation_server::systems::enemy_attack::PendingProjectile>(&mut server),
        0
    );
}

#[test]
fn reset_revives_dead_player_with_respawn_timer() {
    let mut server = build_reset_server(vec![], None);
    server.update();

    let player_entity = spawn_player(&mut server, 1, 5.0, 5.0);

    let mut entity_mut = server.world_mut().entity_mut(player_entity);
    entity_mut.get_mut::<NetPlayer>().unwrap().state = PlayerNetState::Dead;
    entity_mut.get_mut::<NetHealth>().unwrap().current = 0.0;
    entity_mut.insert(RespawnTimer(3.0));

    server.world_mut().resource_mut::<MapResetRequested>().0 = true;
    tick_server_n(&mut server, 30);

    let player = server
        .world()
        .entity(player_entity)
        .get::<NetPlayer>()
        .unwrap();
    assert_eq!(player.state, PlayerNetState::Alive);

    let health = server
        .world()
        .entity(player_entity)
        .get::<NetHealth>()
        .unwrap();
    assert_eq!(health.current, health.max);

    assert!(
        server
            .world()
            .entity(player_entity)
            .get::<RespawnTimer>()
            .is_none(),
        "RespawnTimer should be removed"
    );
}

#[test]
fn reset_clears_flame_tracker() {
    let mut server = build_reset_server(vec![], None);
    server.update();

    spawn_player(&mut server, 1, 5.0, 5.0);

    server
        .world_mut()
        .resource_mut::<FlameActiveTracker>()
        .0
        .insert(PlayerId(1), true);

    assert!(!server.world().resource::<FlameActiveTracker>().0.is_empty());

    server.world_mut().resource_mut::<MapResetRequested>().0 = true;
    tick_server_n(&mut server, 30);

    assert!(
        server.world().resource::<FlameActiveTracker>().0.is_empty(),
        "FlameActiveTracker should be cleared"
    );
}

#[test]
fn reset_clears_fire_cooldowns() {
    let mut server = build_reset_server(vec![], None);
    server.update();

    spawn_player(&mut server, 1, 5.0, 5.0);

    server
        .world_mut()
        .resource_mut::<FireCooldownMap>()
        .0
        .insert(PlayerId(1), 0.5);

    assert!(!server.world().resource::<FireCooldownMap>().0.is_empty());

    server.world_mut().resource_mut::<MapResetRequested>().0 = true;
    tick_server_n(&mut server, 30);

    assert!(
        server.world().resource::<FireCooldownMap>().0.is_empty(),
        "FireCooldownMap should be cleared"
    );
}
