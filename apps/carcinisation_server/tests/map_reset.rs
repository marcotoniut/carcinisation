//! ECS-level map reset integration tests.
//!
//! Tests cover enemy despawn/respawn, player preservation, projectile cleanup,
//! dead-player revival, and per-player resource clearing on reset.
//!
//! Admin socket tests are in `admin_socket.rs`.
#![allow(clippy::float_cmp)]

mod common;

use std::time::Duration;

use bevy::prelude::*;
use carcinisation_fps_core::map::{EntitySpawnData, EntitySpawnKind, test_map};
use carcinisation_fps_core::pickup::PickupKind;
use carcinisation_net::{
    NetAttackId, NetHealth, NetPickup, NetPlayer, NetProjectileType, NetworkObjectId, Owner,
    PlayerId, PlayerNetState,
};
use carcinisation_server::ServerPlugin;
use carcinisation_server::systems::reset::MapResetRequested;
use carcinisation_server::systems::{
    FireCooldownMap, FlameActiveTracker, NetEnemy, NetProjectile, ProjectileTtl, RespawnTimer,
    ServerQuickTurn,
};
use common::{build_server_app, reserve_port};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn build_server(entities: Vec<EntitySpawnData>) -> App {
    let port = reserve_port();
    build_server_app(ServerPlugin {
        port,
        map: test_map(),
        entities,
        player_starts: vec![],
        admin_socket: None,
        instance_name: "test".to_string(),
        map_path: "test_map".to_string(),
    })
}

fn one_enemy() -> Vec<EntitySpawnData> {
    vec![EntitySpawnData {
        x: 3.5,
        y: 1.5,
        kind: EntitySpawnKind::Mosquiton {
            health: 100,
            speed: 0.0,
        },
    }]
}

fn two_pickups() -> Vec<EntitySpawnData> {
    vec![
        EntitySpawnData {
            x: 2.0,
            y: 2.0,
            kind: EntitySpawnKind::Pickup {
                kind: PickupKind::Health,
                respawnable: true,
            },
        },
        EntitySpawnData {
            x: 5.0,
            y: 5.0,
            kind: EntitySpawnKind::Pickup {
                kind: PickupKind::Health,
                respawnable: false,
            },
        },
    ]
}

fn spawn_player(app: &mut App, pid: u32, x: f32, y: f32) -> Entity {
    app.world_mut()
        .spawn((
            NetPlayer {
                player_id: PlayerId(pid),
                position: Vec2::new(x, y),
                angle: 0.0,
                current_attack: NetAttackId::None,
                state: PlayerNetState::Alive,
                flame_active: false,
                avatar_palette_variant: None,
            },
            NetHealth {
                current: 100.0,
                max: 100.0,
            },
            ServerQuickTurn::default(),
        ))
        .id()
}

fn tick_server(app: &mut App, ticks: u32) {
    for _ in 0..ticks {
        std::thread::sleep(Duration::from_millis(2));
        app.update();
    }
}

fn count<C: Component>(app: &mut App) -> usize {
    app.world_mut()
        .query_filtered::<Entity, With<C>>()
        .iter(app.world())
        .count()
}

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
    let mut server = build_server(one_enemy());
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

    tick_server(&mut server, 300);

    server.world_mut().resource_mut::<MapResetRequested>().0 = true;
    tick_server(&mut server, 30);

    assert_eq!(count::<NetEnemy>(&mut server), 1);
}

#[test]
fn reset_despawns_pickups_and_respawns_without_duplicate_ids() {
    let mut server = build_server(two_pickups());
    server.update();

    let ids_before = pickup_object_ids(&mut server);
    assert_eq!(ids_before.len(), 2);
    let unique_before = ids_before
        .iter()
        .copied()
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(unique_before.len(), ids_before.len());

    server.world_mut().resource_mut::<MapResetRequested>().0 = true;
    tick_server(&mut server, 30);

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
    let mut server = build_server(vec![]);
    server.update();

    let player_entity = spawn_player(&mut server, 1, 5.0, 5.0);

    server
        .world_mut()
        .entity_mut(player_entity)
        .get_mut::<NetHealth>()
        .unwrap()
        .current = 10.0;

    server.world_mut().resource_mut::<MapResetRequested>().0 = true;
    tick_server(&mut server, 30);

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
    let mut server = build_server(vec![]);
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
    tick_server(&mut server, 30);

    assert_eq!(count::<NetProjectile>(&mut server), 0);
}

#[test]
fn reset_despawns_pending_projectiles() {
    let mut server = build_server(one_enemy());
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
    tick_server(&mut server, 30);

    assert_eq!(
        count::<carcinisation_server::systems::enemy_attack::PendingProjectile>(&mut server),
        0
    );
}

#[test]
fn reset_revives_dead_player_with_respawn_timer() {
    let mut server = build_server(vec![]);
    server.update();

    let player_entity = spawn_player(&mut server, 1, 5.0, 5.0);

    let mut entity_mut = server.world_mut().entity_mut(player_entity);
    entity_mut.get_mut::<NetPlayer>().unwrap().state = PlayerNetState::Dead;
    entity_mut.get_mut::<NetHealth>().unwrap().current = 0.0;
    entity_mut.insert(RespawnTimer(3.0));

    server.world_mut().resource_mut::<MapResetRequested>().0 = true;
    tick_server(&mut server, 30);

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
    let mut server = build_server(vec![]);
    server.update();

    spawn_player(&mut server, 1, 5.0, 5.0);

    server
        .world_mut()
        .resource_mut::<FlameActiveTracker>()
        .0
        .insert(PlayerId(1), true);

    assert!(!server.world().resource::<FlameActiveTracker>().0.is_empty());

    server.world_mut().resource_mut::<MapResetRequested>().0 = true;
    tick_server(&mut server, 30);

    assert!(
        server.world().resource::<FlameActiveTracker>().0.is_empty(),
        "FlameActiveTracker should be cleared"
    );
}

#[test]
fn reset_clears_fire_cooldowns() {
    let mut server = build_server(vec![]);
    server.update();

    spawn_player(&mut server, 1, 5.0, 5.0);

    server
        .world_mut()
        .resource_mut::<FireCooldownMap>()
        .0
        .insert(PlayerId(1), 0.5);

    assert!(!server.world().resource::<FireCooldownMap>().0.is_empty());

    server.world_mut().resource_mut::<MapResetRequested>().0 = true;
    tick_server(&mut server, 30);

    assert!(
        server.world().resource::<FireCooldownMap>().0.is_empty(),
        "FireCooldownMap should be cleared"
    );
}
