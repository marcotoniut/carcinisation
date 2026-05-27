//! Admin socket round-trip integration tests.
//!
//! Tests cover Unix socket protocol: help, status, players, reset-map, say, unknown commands.
#![allow(clippy::float_cmp)]

mod common;

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;

use bevy::prelude::*;
use carcinisation_fps_core::map::{EntitySpawnData, EntitySpawnKind, test_map};
use carcinisation_net::{NetAttackId, NetHealth, NetPlayer, PlayerId, PlayerNetState};
use carcinisation_server::ServerPlugin;
use carcinisation_server::systems::{NetEnemy, ServerQuickTurn};
use common::{build_server_app, reserve_port};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn build_server(entities: Vec<EntitySpawnData>, admin_socket: String) -> App {
    let port = reserve_port();
    build_server_app(ServerPlugin {
        port,
        map: test_map(),
        entities,
        player_starts: vec![],
        admin_socket: Some(admin_socket),
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

fn spawn_player(app: &mut App, pid: u32, x: f32, y: f32) {
    app.world_mut().spawn((
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
    ));
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

fn unique_socket_path() -> String {
    use std::sync::atomic::{AtomicU32, Ordering};
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    format!(
        "/tmp/carcinisation-test-{}-{n}.admin.sock",
        std::process::id()
    )
}

/// Send a JSON admin request over a Unix socket while ticking the server.
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
    tick_server(server, 30);
    handle.join().expect("admin request thread")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn admin_socket_help() {
    let sock = unique_socket_path();
    let mut server = build_server(vec![], sock.clone());
    tick_server(&mut server, 30);

    let resp = admin_request(&mut server, &sock, &carcinisation_admin::AdminRequest::Help);
    assert!(resp.ok);
    assert!(resp.message.unwrap().contains("help"));
}

#[test]
fn admin_socket_status_with_enemies() {
    let sock = unique_socket_path();
    let mut server = build_server(one_enemy(), sock.clone());
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
    let mut server = build_server(vec![], sock.clone());
    tick_server(&mut server, 30);

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
    let mut server = build_server(one_enemy(), sock.clone());
    tick_server(&mut server, 30);

    let resp = admin_request(
        &mut server,
        &sock,
        &carcinisation_admin::AdminRequest::Status,
    );
    assert_eq!(resp.data.unwrap()["enemies"], 1);

    // Despawn the enemy directly.
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
    let mut server = build_server(vec![], sock.clone());
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
    let mut server = build_server(vec![], sock.clone());
    tick_server(&mut server, 30);

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
