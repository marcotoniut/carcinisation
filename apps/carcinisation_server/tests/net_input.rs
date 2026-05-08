//! Integration tests: fixed-tick server-authoritative input pipeline.
#![allow(clippy::doc_markdown)]
//!
//! All tests run headless (MinimalPlugins) with real UDP transport.
//! `tick_with_sleep` uses a 2 ms sleep so real time accumulates for Bevy's
//! `FixedUpdate` to fire (~every 16–17 ticks at 30 Hz ≈ 33 ms period).
//! This gives UDP packets wall-clock time to arrive before the server's
//! next `FixedUpdate` processes them.
//!
//! Flow: client_trigger → UDP → FromClient observer → PlayerInputBuffer
//!       → FixedUpdate apply_buffered_movement → NetPlayer mutation → replication

mod common;

use std::net::SocketAddr;

use bevy::prelude::*;
use bevy_replicon::prelude::*;
use carcinisation_net::{ClientIntent, InputSequence, NetPlayer, PlayerActions, PlayerId};
use common::{
    build_fixed_tick_client, build_fixed_tick_server, reserve_port, tick_with_sleep,
    tick3_with_sleep,
};

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

/// Queued intents that `send_queued_intents` drains each frame.
#[derive(Resource, Default)]
struct TestIntentQueue(Vec<ClientIntent>);

/// Drains the queue and sends each entry via `client_trigger`.
fn send_queued_intents(mut commands: Commands, mut queue: ResMut<TestIntentQueue>) {
    for intent in queue.0.drain(..) {
        commands.client_trigger(intent);
    }
}

/// Build a test client wired for programmatic intent injection with fixed time stepping.
fn build_input_client(addr: SocketAddr) -> App {
    let mut app = build_fixed_tick_client(addr);
    app.init_resource::<TestIntentQueue>();
    app.add_systems(Update, send_queued_intents);
    app
}

/// Push a forward movement intent into the client's queue.
fn queue_forward(app: &mut App, seq: u32) {
    app.world_mut()
        .resource_mut::<TestIntentQueue>()
        .0
        .push(ClientIntent {
            sequence: InputSequence(seq),
            movement: Vec2::new(0.0, 1.0),
            turn: 0.0,
            fire_held: false,
            actions: PlayerActions::default(),
        });
}

/// Push an idle intent (no movement/fire).
fn queue_idle(app: &mut App, seq: u32) {
    app.world_mut()
        .resource_mut::<TestIntentQueue>()
        .0
        .push(ClientIntent::idle(InputSequence(seq)));
}

/// Push a turn-left intent.
fn queue_turn_left(app: &mut App, seq: u32) {
    app.world_mut()
        .resource_mut::<TestIntentQueue>()
        .0
        .push(ClientIntent {
            sequence: InputSequence(seq),
            movement: Vec2::ZERO,
            turn: 1.0,
            fire_held: false,
            actions: PlayerActions::default(),
        });
}

/// Collect server-side `(PlayerId, position)` pairs, sorted by id.
fn get_server_players(server: &mut App) -> Vec<(PlayerId, Vec2)> {
    let mut result: Vec<_> = server
        .world_mut()
        .query::<&NetPlayer>()
        .iter(server.world())
        .map(|p| (p.player_id, p.position))
        .collect();
    result.sort_by_key(|(id, _)| id.0);
    result
}

/// Tick server + one client.
fn tick2(server: &mut App, client: &mut App) {
    server.update();
    client.update();
}

/// Block until `n` NetPlayers exist on the server (max 200 frames).
fn wait_for_players_2(n: usize, server: &mut App, client: &mut App) -> bool {
    for _ in 0..200 {
        tick2(server, client);
        let count = server
            .world_mut()
            .query::<&NetPlayer>()
            .iter(server.world())
            .count();
        if count >= n {
            return true;
        }
    }
    false
}

/// Block until `n` NetPlayers exist on the server (max 200 frames, two clients).
fn wait_for_players_3(n: usize, server: &mut App, c1: &mut App, c2: &mut App) -> bool {
    for _ in 0..200 {
        server.update();
        c1.update();
        c2.update();
        let count = server
            .world_mut()
            .query::<&NetPlayer>()
            .iter(server.world())
            .count();
        if count >= n {
            return true;
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Single client sends BTN_FORWARD → server moves the correct NetPlayer.
#[test]
fn client_input_moves_own_player() {
    let port = reserve_port();
    let mut server = build_fixed_tick_server(port);
    server.update();

    let addr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), port);
    let mut client = build_input_client(addr);
    client.update();

    assert!(
        wait_for_players_2(1, &mut server, &mut client),
        "NetPlayer should exist after connection"
    );

    let initial = get_server_players(&mut server);
    assert_eq!(initial.len(), 1);
    let start_pos = initial[0].1;

    // Send forward input for several ticks.
    for seq in 1..=5 {
        queue_forward(&mut client, seq);
        tick_with_sleep(&mut server, &mut client);
    }

    // Extra ticks for network propagation + FixedUpdate processing.
    for _ in 0..30 {
        tick_with_sleep(&mut server, &mut client);
    }

    let final_players = get_server_players(&mut server);
    assert_eq!(final_players.len(), 1);
    assert!(
        final_players[0].1.x > start_pos.x,
        "Player should have moved forward (+X at angle=0): start={start_pos:?} end={:?}",
        final_players[0].1
    );
}

/// Client 1 sends input. Client 2 does not.
/// Only Player 1 should move; Player 2 must stay put.
#[test]
fn only_sender_moves() {
    let port = reserve_port();
    let mut server = build_fixed_tick_server(port);
    server.update();

    let addr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), port);
    let mut c1 = build_input_client(addr);
    c1.update();
    std::thread::sleep(std::time::Duration::from_millis(2));
    let mut c2 = build_input_client(addr);
    c2.update();

    assert!(
        wait_for_players_3(2, &mut server, &mut c1, &mut c2),
        "Both NetPlayers should spawn"
    );

    let initial = get_server_players(&mut server);
    assert_eq!(initial.len(), 2, "Exactly 2 players expected");
    let p1_start = initial[0].1;
    let p2_start = initial[1].1;

    // Only client 1 sends forward.
    for seq in 1..=5 {
        queue_forward(&mut c1, seq);
        tick3_with_sleep(&mut server, &mut c1, &mut c2);
    }
    for _ in 0..30 {
        tick3_with_sleep(&mut server, &mut c1, &mut c2);
    }

    let after = get_server_players(&mut server);
    assert_eq!(after.len(), 2);

    assert!(
        after[0].1.x > p1_start.x,
        "Player 1 should have moved: {p1_start:?} → {:?}",
        after[0].1
    );
    assert_eq!(
        after[1].1, p2_start,
        "Player 2 must not move: {p2_start:?} → {:?}",
        after[1].1
    );
}

/// Both clients send input simultaneously in different directions.
/// Player 1 moves forward (+X at angle=0), Player 2 strafes left (+Y at angle=0).
/// No cross-contamination.
#[test]
fn simultaneous_input_isolated() {
    let port = reserve_port();
    let mut server = build_fixed_tick_server(port);
    server.update();

    let addr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), port);
    let mut c1 = build_input_client(addr);
    c1.update();
    std::thread::sleep(std::time::Duration::from_millis(2));
    let mut c2 = build_input_client(addr);
    c2.update();

    assert!(
        wait_for_players_3(2, &mut server, &mut c1, &mut c2),
        "Both NetPlayers should spawn"
    );

    let initial = get_server_players(&mut server);
    assert_eq!(initial.len(), 2);
    let p1_start = initial[0].1;
    let p2_start = initial[1].1;

    // c1: forward, c2: turn left (continuous).
    // At angle=0: forward → world +X; turn left changes angle.
    for seq in 1..=10 {
        queue_forward(&mut c1, seq);
        queue_turn_left(&mut c2, seq);
        tick3_with_sleep(&mut server, &mut c1, &mut c2);
    }
    for _ in 0..30 {
        tick3_with_sleep(&mut server, &mut c1, &mut c2);
    }

    let final_players = get_server_players(&mut server);
    assert_eq!(final_players.len(), 2);
    let p1_end = final_players[0].1;
    let p2_end = final_players[1].1;

    // Player 1: moved forward (+X at angle=0), y unchanged.
    assert!(
        p1_end.x > p1_start.x,
        "P1 x should increase (forward): {:.2} → {:.2}",
        p1_start.x,
        p1_end.x
    );
    assert!(
        (p1_end.y - p1_start.y).abs() < 0.01,
        "P1 y should stay: {:.2} → {:.2}",
        p1_start.y,
        p1_end.y
    );

    // Player 2: turned left — position unchanged, angle changed.
    assert!(
        (p2_end.x - p2_start.x).abs() < 0.01 && (p2_end.y - p2_start.y).abs() < 0.01,
        "P2 position should stay (turn only): {p2_start:?} → {p2_end:?}"
    );
}

/// Held input persists across ticks without re-sending.
/// Send BTN_FORWARD once, then tick the server multiple times without new input.
/// Player should keep moving because buffer retains the latest buttons.
#[test]
fn held_input_persists_across_ticks() {
    let port = reserve_port();
    let mut server = build_fixed_tick_server(port);
    server.update();

    let addr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), port);
    let mut client = build_input_client(addr);
    client.update();

    assert!(
        wait_for_players_2(1, &mut server, &mut client),
        "NetPlayer should exist"
    );

    let start_pos = get_server_players(&mut server)[0].1;

    // Send forward a few times to ensure at least one arrives.
    for seq in 1..=3 {
        queue_forward(&mut client, seq);
        for _ in 0..5 {
            tick_with_sleep(&mut server, &mut client);
        }
    }

    // Confirm the input arrived — position should have moved from start.
    let after_send = get_server_players(&mut server)[0].1;
    assert!(
        after_send.x > start_pos.x,
        "Input should have arrived: start={:.2} now={:.2}",
        start_pos.x,
        after_send.x
    );

    // Tick 30 more times WITHOUT sending new input.
    // Buffer should retain BTN_FORWARD → player keeps moving.
    for _ in 0..30 {
        tick_with_sleep(&mut server, &mut client);
    }

    let after_hold = get_server_players(&mut server)[0].1;
    assert!(
        after_hold.x > after_send.x + 0.05,
        "Player should keep moving after initial send: {:.2} → {:.2}",
        after_send.x,
        after_hold.x
    );
}

/// Sending buttons=0 stops movement.
#[test]
fn buttons_zero_stops_movement() {
    let port = reserve_port();
    let mut server = build_fixed_tick_server(port);
    server.update();

    let addr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), port);
    let mut client = build_input_client(addr);
    client.update();

    assert!(
        wait_for_players_2(1, &mut server, &mut client),
        "NetPlayer should exist"
    );

    // Start moving forward.
    queue_forward(&mut client, 1);
    for _ in 0..10 {
        tick_with_sleep(&mut server, &mut client);
    }

    // Send buttons=0 to stop.
    queue_idle(&mut client, 2);
    for _ in 0..10 {
        tick_with_sleep(&mut server, &mut client);
    }

    let stopped_pos = get_server_players(&mut server)[0].1;

    // Tick more — position should not change.
    for _ in 0..20 {
        tick_with_sleep(&mut server, &mut client);
    }

    let final_pos = get_server_players(&mut server)[0].1;
    assert_eq!(
        stopped_pos, final_pos,
        "Player should not move after buttons=0: {stopped_pos:?} → {final_pos:?}"
    );
}

/// Duplicate sequence numbers are rejected by the observer.
/// The buffer is NOT updated by a duplicate, so whatever was last valid stays.
#[test]
fn duplicate_sequence_rejected() {
    let port = reserve_port();
    let mut server = build_fixed_tick_server(port);
    server.update();

    let addr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), port);
    let mut client = build_input_client(addr);
    client.update();

    assert!(
        wait_for_players_2(1, &mut server, &mut client),
        "NetPlayer should exist"
    );

    // seq=1: move forward.
    queue_forward(&mut client, 1);
    for _ in 0..5 {
        tick_with_sleep(&mut server, &mut client);
    }

    // seq=2: stop (buttons=0).
    queue_idle(&mut client, 2);
    for _ in 0..5 {
        tick_with_sleep(&mut server, &mut client);
    }

    let stopped_pos = get_server_players(&mut server)[0].1;

    // Now send seq=2 again with BTN_FORWARD — should be REJECTED (duplicate).
    // Buffer should remain at buttons=0, player should not move.
    queue_forward(&mut client, 2);
    for _ in 0..20 {
        tick_with_sleep(&mut server, &mut client);
    }

    let after_dup = get_server_players(&mut server)[0].1;
    assert_eq!(
        stopped_pos, after_dup,
        "Duplicate seq should not change buffer: {stopped_pos:?} → {after_dup:?}"
    );
}

/// Out-of-order (lower) sequence numbers are rejected by the observer.
/// Sending seq=3 then seq=1 should not revert to the seq=1 intent.
#[test]
fn out_of_order_sequence_rejected() {
    let port = reserve_port();
    let mut server = build_fixed_tick_server(port);
    server.update();

    let addr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), port);
    let mut client = build_input_client(addr);
    client.update();

    assert!(
        wait_for_players_2(1, &mut server, &mut client),
        "NetPlayer should exist"
    );

    // seq=1: move forward.
    queue_forward(&mut client, 1);
    for _ in 0..5 {
        tick_with_sleep(&mut server, &mut client);
    }

    // seq=3: stop (skip seq=2 to simulate reordering).
    queue_idle(&mut client, 3);
    for _ in 0..5 {
        tick_with_sleep(&mut server, &mut client);
    }

    let stopped_pos = get_server_players(&mut server)[0].1;

    // Now send seq=1 again with forward — should be rejected (older than last=3).
    queue_forward(&mut client, 1);
    for _ in 0..20 {
        tick_with_sleep(&mut server, &mut client);
    }

    let after_old = get_server_players(&mut server)[0].1;
    assert_eq!(
        stopped_pos, after_old,
        "Out-of-order seq should not change buffer: {stopped_pos:?} → {after_old:?}"
    );
}
