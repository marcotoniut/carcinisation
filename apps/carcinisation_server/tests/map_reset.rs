//! Integration tests for admin commands and map reset.
//!
//! Tests cover:
//! - Enemy despawn/respawn across reset
//! - Player preservation with health restoration
//! - Projectile despawn
//! - `PendingProjectile` despawn
//! - Dead player (with `RespawnTimer`) reset to alive
//! - `FlameActiveTracker` cleared on reset
//! - `FireCooldownMap` cleared on reset
//! - Admin socket round-trip for all commands
//! - Admin socket `players` with connected player data
//! - Admin socket `status` enemy count after reset-map

#![allow(clippy::float_cmp)]

mod common;

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;

use bevy::prelude::*;
use carcinisation_fps_core::map::{EntitySpawnData, EntitySpawnKind, test_map};
use carcinisation_net::{
    NetAttackId, NetHealth, NetPlayer, NetProjectileType, NetworkObjectId, Owner, PlayerId,
    PlayerNetState,
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

fn build_server(entities: Vec<EntitySpawnData>, admin_socket: Option<String>) -> App {
    let port = reserve_port();
    build_server_app(ServerPlugin {
        port,
        map: test_map(),
        entities,
        player_starts: vec![],
        admin_socket,
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

/// Send a JSON admin request over a Unix socket while ticking the server.
/// The request is sent from a background thread; the main thread ticks
/// the server so `poll_admin_socket` runs and produces a response.
fn admin_request(
    server: &mut App,
    socket_path: &str,
    request: &carcinisation_admin::AdminRequest,
) -> carcinisation_admin::AdminResponse {
    let sock = socket_path.to_string();
    let req = request.clone();
    let handle = std::thread::spawn(move || {
        let mut stream = UnixStream::connect(&sock).expect("connect to admin socket");
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let payload = serde_json::to_string(&req).unwrap();
        writeln!(stream, "{payload}").expect("send request");
        let _ = stream.shutdown(std::net::Shutdown::Write);
        let mut reader = BufReader::new(&stream);
        let mut line = String::new();
        reader.read_line(&mut line).expect("read response");
        serde_json::from_str::<carcinisation_admin::AdminResponse>(line.trim())
            .expect("parse response")
    });
    // Tick server so FixedUpdate fires and processes the socket.
    tick_server(server, 30);
    handle.join().expect("admin request thread")
}

// ---------------------------------------------------------------------------
// ECS-level tests (no admin socket)
// ---------------------------------------------------------------------------

#[test]
fn reset_despawns_enemies_and_respawns_them() {
    let mut server = build_server(one_enemy(), None);
    server.update();
    assert_eq!(count::<NetEnemy>(&mut server), 1);

    // Kill enemy.
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

    // Tick so death/despawn timers fire.
    tick_server(&mut server, 300);

    // Reset.
    server.world_mut().resource_mut::<MapResetRequested>().0 = true;
    tick_server(&mut server, 30);

    assert_eq!(count::<NetEnemy>(&mut server), 1);
}

#[test]
fn reset_preserves_players_and_restores_health() {
    let mut server = build_server(vec![], None);
    server.update();

    let player_entity = spawn_player(&mut server, 1, 5.0, 5.0);

    // Damage player.
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
    let mut server = build_server(vec![], None);
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
    let mut server = build_server(one_enemy(), None);
    server.update();

    // Get the enemy entity to use as source_entity.
    let enemy_entity = server
        .world_mut()
        .query_filtered::<Entity, With<NetEnemy>>()
        .iter(server.world())
        .next()
        .unwrap();

    // Spawn a PendingProjectile (simulating enemy attack wind-up).
    server.world_mut().spawn(
        carcinisation_server::systems::enemy_attack::PendingProjectile {
            timer: 1.0,
            source_entity: enemy_entity,
            position: Vec2::new(3.5, 1.5),
            angle: 0.0,
            damage: 10.0,
            object_id: NetworkObjectId(500),
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
    let mut server = build_server(vec![], None);
    server.update();

    let player_entity = spawn_player(&mut server, 1, 5.0, 5.0);

    // Simulate dead state with RespawnTimer.
    let mut entity_mut = server.world_mut().entity_mut(player_entity);
    entity_mut.get_mut::<NetPlayer>().unwrap().state = PlayerNetState::Dead;
    entity_mut.get_mut::<NetHealth>().unwrap().current = 0.0;
    entity_mut.insert(RespawnTimer(3.0));

    server.world_mut().resource_mut::<MapResetRequested>().0 = true;
    tick_server(&mut server, 30);

    // Player should be alive with full health, RespawnTimer removed.
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
    let mut server = build_server(vec![], None);
    server.update();

    spawn_player(&mut server, 1, 5.0, 5.0);

    // Set flame tracker to active for player 1.
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
    let mut server = build_server(vec![], None);
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

// ---------------------------------------------------------------------------
// Admin socket round-trip tests
// ---------------------------------------------------------------------------

fn unique_socket_path() -> String {
    use std::sync::atomic::{AtomicU32, Ordering};
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    format!(
        "/tmp/carcinisation-test-{}-{n}.admin.sock",
        std::process::id()
    )
}

#[test]
fn admin_socket_help() {
    let sock = unique_socket_path();
    let mut server = build_server(vec![], Some(sock.clone()));
    tick_server(&mut server, 30);

    let resp = admin_request(&mut server, &sock, &carcinisation_admin::AdminRequest::Help);
    assert!(resp.ok);
    assert!(resp.message.unwrap().contains("help"));
}

#[test]
fn admin_socket_status_with_enemies() {
    let sock = unique_socket_path();
    let mut server = build_server(one_enemy(), Some(sock.clone()));
    tick_server(&mut server, 30);

    let resp = admin_request(
        &mut server,
        &sock,
        &carcinisation_admin::AdminRequest::Status,
    );
    assert!(resp.ok);
    let data = resp.data.unwrap();
    assert_eq!(data["instance"], "test");
    assert_eq!(data["enemies"], 1);
    assert_eq!(data["players"], 0);
}

#[test]
fn admin_socket_players_with_connected_player() {
    let sock = unique_socket_path();
    let mut server = build_server(vec![], Some(sock.clone()));
    tick_server(&mut server, 30);

    // Spawn a player (simulates connection).
    spawn_player(&mut server, 42, 3.0, 4.0);
    tick_server(&mut server, 10);

    let resp = admin_request(
        &mut server,
        &sock,
        &carcinisation_admin::AdminRequest::Players,
    );
    assert!(resp.ok);
    assert!(resp.message.unwrap().contains("1 player(s) connected"));
    let data = resp.data.unwrap();
    let players = data.as_array().unwrap();
    assert_eq!(players.len(), 1);
    assert_eq!(players[0]["player_id"], 42);
    assert_eq!(players[0]["state"], "Alive");
}

#[test]
fn admin_socket_reset_map_despawns_and_respawns() {
    let sock = unique_socket_path();
    let mut server = build_server(one_enemy(), Some(sock.clone()));
    tick_server(&mut server, 30);

    // Confirm 1 enemy via status.
    let resp = admin_request(
        &mut server,
        &sock,
        &carcinisation_admin::AdminRequest::Status,
    );
    assert_eq!(resp.data.unwrap()["enemies"], 1);

    // Despawn the enemy directly (simulating death + despawn completion).
    let enemy_entity = server
        .world_mut()
        .query_filtered::<Entity, With<NetEnemy>>()
        .iter(server.world())
        .next()
        .unwrap();
    server.world_mut().despawn(enemy_entity);
    tick_server(&mut server, 10);
    assert_eq!(count::<NetEnemy>(&mut server), 0);

    // Reset via admin socket.
    let resp = admin_request(
        &mut server,
        &sock,
        &carcinisation_admin::AdminRequest::ResetMap,
    );
    assert!(resp.ok);
    tick_server(&mut server, 30);

    // Verify enemy respawned via status.
    let resp = admin_request(
        &mut server,
        &sock,
        &carcinisation_admin::AdminRequest::Status,
    );
    assert_eq!(resp.data.unwrap()["enemies"], 1);
}

#[test]
fn admin_socket_say_returns_not_implemented() {
    let sock = unique_socket_path();
    let mut server = build_server(vec![], Some(sock.clone()));
    tick_server(&mut server, 30);

    let resp = admin_request(
        &mut server,
        &sock,
        &carcinisation_admin::AdminRequest::Say {
            message: "hello".to_string(),
        },
    );
    assert!(!resp.ok);
    assert!(resp.error.unwrap().contains("not implemented"));
}

#[test]
fn admin_socket_unknown_command_rejected() {
    let sock = unique_socket_path();
    let mut server = build_server(vec![], Some(sock.clone()));
    tick_server(&mut server, 30);

    // Send raw malformed JSON from a background thread.
    let sock2 = sock.clone();
    let handle = std::thread::spawn(move || {
        let mut stream = UnixStream::connect(&sock2).expect("connect");
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        writeln!(stream, r#"{{"command":"explode"}}"#).unwrap();
        let _ = stream.shutdown(std::net::Shutdown::Write);
        let mut reader = BufReader::new(&stream);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        serde_json::from_str::<carcinisation_admin::AdminResponse>(line.trim()).unwrap()
    });
    tick_server(&mut server, 30);
    let resp = handle.join().expect("thread");
    assert!(!resp.ok);
    assert!(resp.error.unwrap().contains("invalid request"));

    // Server should still be alive.
    tick_server(&mut server, 10);
}
