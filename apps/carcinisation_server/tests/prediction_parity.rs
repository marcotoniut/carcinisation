//! Prediction–server parity integration test.
//!
//! Verifies the full UDP pipeline: client sends `ClientIntent`, server
//! processes it in `FixedUpdate`, sends `InputAck` with authoritative
//! position/angle, and client receives the ack. Then validates that
//! running the same `apply_movement` leaf function from ack state
//! produces positions consistent with subsequent acks.
//!
//! This is the highest-value regression test for the prediction system:
//! if movement config, collision math, or ack generation diverge, these
//! tests will catch it.
#![allow(clippy::doc_markdown)]

mod common;

use std::net::SocketAddr;

use bevy::prelude::*;
use bevy_replicon::prelude::*;
use carcinisation_fps_core::FpsMovementConfig;
use carcinisation_fps_core::map::test_map;
use carcinisation_fps_core::movement::apply_movement;
use carcinisation_fps_core::movement::tick_snap_turn;
use carcinisation_net::{ClientIntent, InputAck, InputSequence, NetPlayer, PlayerActions};
use common::{build_fixed_tick_client, build_fixed_tick_server, reserve_port, tick_with_sleep};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

#[derive(Resource, Default)]
struct TestIntentQueue(Vec<ClientIntent>);

fn send_queued_intents(mut commands: Commands, mut queue: ResMut<TestIntentQueue>) {
    for intent in queue.0.drain(..) {
        commands.client_trigger(intent);
    }
}

/// Capture InputAck events on the client side.
#[derive(Resource, Default)]
struct CapturedAcks(Vec<InputAck>);

#[allow(clippy::needless_pass_by_value)]
fn capture_input_acks(trigger: On<InputAck>, mut acks: ResMut<CapturedAcks>) {
    acks.0.push(trigger.event().clone());
}

fn build_prediction_client(addr: SocketAddr) -> App {
    let mut app = build_fixed_tick_client(addr);
    app.init_resource::<TestIntentQueue>();
    app.init_resource::<CapturedAcks>();
    app.add_observer(capture_input_acks);
    app.add_systems(Update, send_queued_intents);
    app
}

fn queue_intent(app: &mut App, seq: u32, movement: Vec2, turn: f32) {
    queue_intent_with_actions(app, seq, movement, turn, PlayerActions::default());
}

fn queue_intent_with_actions(
    app: &mut App,
    seq: u32,
    movement: Vec2,
    turn: f32,
    actions: PlayerActions,
) {
    app.world_mut()
        .resource_mut::<TestIntentQueue>()
        .0
        .push(ClientIntent {
            sequence: InputSequence(seq),
            movement,
            turn,
            fire_held: false,
            aim_held: false,
            aim_offset: 0.0,
            actions,
        });
}

fn get_server_player(server: &mut App) -> Option<(Vec2, f32)> {
    server
        .world_mut()
        .query::<&NetPlayer>()
        .iter(server.world())
        .next()
        .map(|p| (p.position, p.angle))
}

fn wait_for_player(server: &mut App, client: &mut App) -> bool {
    for _ in 0..200 {
        server.update();
        client.update();
        if server
            .world_mut()
            .query::<&NetPlayer>()
            .iter(server.world())
            .count()
            >= 1
        {
            return true;
        }
    }
    false
}

fn drain_acks(client: &mut App) -> Vec<InputAck> {
    let mut acks = client.world_mut().resource_mut::<CapturedAcks>();
    std::mem::take(&mut acks.0)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Full pipeline: client sends forward intents → server moves player →
/// InputAck arrives with authoritative position east of spawn.
#[test]
fn ack_carries_authoritative_position_after_forward() {
    let port = reserve_port();
    let mut server = build_fixed_tick_server(port);
    server.update();

    let addr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), port);
    let mut client = build_prediction_client(addr);
    client.update();

    assert!(
        wait_for_player(&mut server, &mut client),
        "NetPlayer should exist"
    );

    let (start_pos, start_angle) = get_server_player(&mut server).unwrap();

    // Send forward intents over many ticks. Use more ticks than the
    // minimum so several FixedUpdate cycles fire.
    for seq in 1..=30u32 {
        queue_intent(&mut client, seq, Vec2::new(0.0, 1.0), 0.0);
        tick_with_sleep(&mut server, &mut client);
    }

    // Extra ticks for propagation + ack delivery.
    for _ in 0..80 {
        tick_with_sleep(&mut server, &mut client);
    }

    let acks = drain_acks(&mut client);
    assert!(
        !acks.is_empty(),
        "should have received at least one InputAck"
    );

    // Verify acks carry positions east of spawn (angle=0 → forward = +X).
    let last_ack = acks.last().unwrap();
    assert!(
        last_ack.position.x > start_pos.x,
        "ack position should be east of spawn: spawn={start_pos:?} ack={:?}",
        last_ack.position
    );

    // Ack angle should be unchanged (no turn input).
    assert!(
        (last_ack.angle - start_angle).abs() < 0.01,
        "angle should be unchanged: start={start_angle:.3} ack={:.3}",
        last_ack.angle
    );

    // Verify ack position is consistent with apply_movement math:
    // starting from spawn, apply N ticks of forward movement where
    // N = number of acks (each ack = one FixedUpdate tick of movement).
    let cfg = FpsMovementConfig::load();
    let map = test_map();
    let dt = 1.0 / 30.0;
    let mut sim_pos = start_pos;
    for _ in 0..acks.len() {
        apply_movement(
            &mut sim_pos,
            start_angle,
            Vec2::new(0.0, 1.0),
            cfg.move_speed,
            dt,
            &map,
            cfg.collision_margin,
        );
    }
    // Simulated and last ack should be in the same ballpark.
    // Not exact because some FixedUpdate ticks may have fired before
    // the first intent arrived (idle ticks produce no movement).
    let drift = (sim_pos - last_ack.position).length();
    assert!(
        drift < 0.5,
        "ack position should be reachable via apply_movement: \
         sim={sim_pos:?} ack={:?} drift={drift:.3} ({} acks)",
        last_ack.position,
        acks.len()
    );
}

/// Full pipeline: turn-left intents → ack angle increases, position unchanged.
#[test]
fn ack_carries_authoritative_angle_after_turn() {
    let port = reserve_port();
    let mut server = build_fixed_tick_server(port);
    server.update();

    let addr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), port);
    let mut client = build_prediction_client(addr);
    client.update();

    assert!(
        wait_for_player(&mut server, &mut client),
        "NetPlayer should exist"
    );

    let (start_pos, _start_angle) = get_server_player(&mut server).unwrap();

    for seq in 1..=30u32 {
        queue_intent(&mut client, seq, Vec2::ZERO, 1.0);
        tick_with_sleep(&mut server, &mut client);
    }

    for _ in 0..80 {
        tick_with_sleep(&mut server, &mut client);
    }

    let acks = drain_acks(&mut client);
    assert!(!acks.is_empty(), "should have received acks");

    let last_ack = acks.last().unwrap();

    // Angle should have increased (turn=1.0 = left = positive).
    assert!(
        last_ack.angle > 0.05,
        "ack angle should have increased from 0: {:.3}",
        last_ack.angle
    );

    // Position should be nearly unchanged (no movement input).
    assert!(
        (last_ack.position - start_pos).length() < 0.01,
        "position should be unchanged: start={start_pos:?} ack={:?}",
        last_ack.position
    );
}

/// Consecutive acks are consistent: each ack advances exactly one
/// FixedUpdate tick's worth of `apply_movement` from the previous ack.
///
/// This validates that the server's `apply_buffered_movement` uses the
/// same `apply_movement` function and config as the client would.
#[test]
fn consecutive_acks_one_tick_apart() {
    let port = reserve_port();
    let mut server = build_fixed_tick_server(port);
    server.update();

    let addr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), port);
    let mut client = build_prediction_client(addr);
    client.update();

    assert!(
        wait_for_player(&mut server, &mut client),
        "NetPlayer should exist"
    );

    // Load actual config (not ::default) so the test matches the server's values.
    let cfg = FpsMovementConfig::load();
    let map = test_map();
    let dt = 1.0 / 30.0;

    // Send continuous forward input.
    for seq in 1..=40u32 {
        queue_intent(&mut client, seq, Vec2::new(0.0, 1.0), 0.0);
        tick_with_sleep(&mut server, &mut client);
    }

    for _ in 0..80 {
        tick_with_sleep(&mut server, &mut client);
    }

    let acks = drain_acks(&mut client);

    // Need at least 2 consecutive acks to compare.
    if acks.len() < 2 {
        return;
    }

    // Each consecutive pair of acks represents one FixedUpdate tick
    // (send_input_acks runs once per FixedUpdate). Between them, the
    // server applied exactly one `apply_movement` call.
    let mut parity_checked = 0u32;
    for window in acks.windows(2) {
        let prev = &window[0];
        let next = &window[1];

        // Skip if not advancing (shouldn't happen, but defensive).
        if !next
            .last_processed_sequence
            .is_after(prev.last_processed_sequence)
        {
            continue;
        }

        // One tick of apply_movement from prev position.
        let mut sim_pos = prev.position;
        apply_movement(
            &mut sim_pos,
            prev.angle,
            Vec2::new(0.0, 1.0),
            cfg.move_speed,
            dt,
            &map,
            cfg.collision_margin,
        );

        let drift = (sim_pos - next.position).length();
        assert!(
            drift < 0.01,
            "consecutive ack parity: seq {:?} → {:?}, drift={drift:.4}\n\
             prev_pos={:?} sim_pos={sim_pos:?} next_ack_pos={:?}",
            prev.last_processed_sequence,
            next.last_processed_sequence,
            prev.position,
            next.position
        );
        parity_checked += 1;
    }

    assert!(
        parity_checked > 0,
        "should have at least one consecutive ack pair ({} acks)",
        acks.len()
    );
}

/// Ack sequence numbers advance monotonically.
#[test]
fn ack_sequences_advance_monotonically() {
    let port = reserve_port();
    let mut server = build_fixed_tick_server(port);
    server.update();

    let addr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), port);
    let mut client = build_prediction_client(addr);
    client.update();

    assert!(
        wait_for_player(&mut server, &mut client),
        "NetPlayer should exist"
    );

    for seq in 1..=15u32 {
        queue_intent(&mut client, seq, Vec2::new(0.0, 1.0), 0.0);
        tick_with_sleep(&mut server, &mut client);
    }

    for _ in 0..60 {
        tick_with_sleep(&mut server, &mut client);
    }

    let acks = drain_acks(&mut client);
    assert!(!acks.is_empty(), "should have acks");

    for window in acks.windows(2) {
        assert!(
            window[1]
                .last_processed_sequence
                .is_after(window[0].last_processed_sequence)
                || window[1].last_processed_sequence == window[0].last_processed_sequence,
            "ack sequences should be monotonically non-decreasing: {:?} → {:?}",
            window[0].last_processed_sequence,
            window[1].last_processed_sequence
        );
    }
}

/// Mixed inputs: forward + turn. Ack positions move away from spawn.
#[test]
fn mixed_movement_and_turn_ack_plausible() {
    let port = reserve_port();
    let mut server = build_fixed_tick_server(port);
    server.update();

    let addr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), port);
    let mut client = build_prediction_client(addr);
    client.update();

    assert!(
        wait_for_player(&mut server, &mut client),
        "NetPlayer should exist"
    );

    let (start_pos, _) = get_server_player(&mut server).unwrap();

    // Forward + left turn: should arc away from spawn.
    for seq in 1..=30u32 {
        queue_intent(&mut client, seq, Vec2::new(0.0, 1.0), 0.5);
        tick_with_sleep(&mut server, &mut client);
    }

    for _ in 0..80 {
        tick_with_sleep(&mut server, &mut client);
    }

    let acks = drain_acks(&mut client);
    assert!(!acks.is_empty(), "should have acks");

    let last_ack = acks.last().unwrap();

    // Should have moved away from start.
    let distance = (last_ack.position - start_pos).length();
    assert!(
        distance > 0.05,
        "should have moved: start={start_pos:?} ack={:?} distance={distance:.3}",
        last_ack.position
    );

    // Angle should have increased (turning left).
    assert!(
        last_ack.angle > 0.01,
        "angle should have increased: {:.3}",
        last_ack.angle
    );
}

/// Quick turn: ack carries non-zero snap state mid-animation.
///
/// Sends a QUICK_TURN action then idles. Verifies that at least one ack
/// has snap_remaining_radians > 0 (proving the server carries snap state).
#[test]
fn snap_turn_ack_carries_snap_state() {
    let port = reserve_port();
    let mut server = build_fixed_tick_server(port);
    server.update();

    let addr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), port);
    let mut client = build_prediction_client(addr);
    client.update();

    assert!(
        wait_for_player(&mut server, &mut client),
        "NetPlayer should exist"
    );

    // Send QUICK_TURN, then keep sending idle intents with advancing
    // sequences throughout the snap duration. send_input_acks only fires
    // when the sequence advances, so we must keep sending.
    queue_intent_with_actions(
        &mut client,
        1,
        Vec2::ZERO,
        0.0,
        PlayerActions::from_raw(PlayerActions::QUICK_TURN),
    );
    tick_with_sleep(&mut server, &mut client);

    // Keep sending for ~600ms wall time (covers 12-tick snap + propagation).
    for seq in 2..=300u32 {
        queue_intent(&mut client, seq, Vec2::ZERO, 0.0);
        tick_with_sleep(&mut server, &mut client);
    }

    let acks = drain_acks(&mut client);
    assert!(!acks.is_empty(), "should have acks");

    // At least one ack should have snap_remaining > 0 (mid-snap).
    let has_mid_snap = acks.iter().any(|a| a.snap_remaining_radians > 0.01);
    assert!(
        has_mid_snap,
        "at least one ack should carry mid-snap state (all snap_remaining: {:?})",
        acks.iter()
            .map(|a| a.snap_remaining_radians)
            .collect::<Vec<_>>()
    );

    // The last ack should have snap completed (remaining ≈ 0).
    let last = acks.last().unwrap();
    assert!(
        last.snap_remaining_radians < 0.01,
        "last ack should have snap complete: remaining={:.3}",
        last.snap_remaining_radians
    );

    // Angle should be approximately PI from start (180° quick turn).
    assert!(
        last.angle > 2.5,
        "angle should be near PI after quick turn: {:.3}",
        last.angle
    );
}

/// Consecutive acks during a snap turn have monotonically decreasing
/// snap_remaining_radians (or zero when complete).
#[test]
fn snap_remaining_decreases_across_acks() {
    let port = reserve_port();
    let mut server = build_fixed_tick_server(port);
    server.update();

    let addr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), port);
    let mut client = build_prediction_client(addr);
    client.update();

    assert!(
        wait_for_player(&mut server, &mut client),
        "NetPlayer should exist"
    );

    queue_intent_with_actions(
        &mut client,
        1,
        Vec2::ZERO,
        0.0,
        PlayerActions::from_raw(PlayerActions::QUICK_TURN),
    );
    tick_with_sleep(&mut server, &mut client);

    for seq in 2..=300u32 {
        queue_intent(&mut client, seq, Vec2::ZERO, 0.0);
        tick_with_sleep(&mut server, &mut client);
    }

    let acks = drain_acks(&mut client);

    for window in acks.windows(2) {
        assert!(
            window[1].snap_remaining_radians <= window[0].snap_remaining_radians + 0.01,
            "snap_remaining should decrease: seq {:?} ({:.3}) → seq {:?} ({:.3})",
            window[0].last_processed_sequence,
            window[0].snap_remaining_radians,
            window[1].last_processed_sequence,
            window[1].snap_remaining_radians,
        );
    }
}

/// Consecutive snap acks: angle between acks matches tick_snap_turn applied
/// once from the previous ack's snap state.
#[test]
fn consecutive_snap_acks_match_tick_snap_turn() {
    let port = reserve_port();
    let mut server = build_fixed_tick_server(port);
    server.update();

    let addr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), port);
    let mut client = build_prediction_client(addr);
    client.update();

    assert!(
        wait_for_player(&mut server, &mut client),
        "NetPlayer should exist"
    );

    queue_intent_with_actions(
        &mut client,
        1,
        Vec2::ZERO,
        0.0,
        PlayerActions::from_raw(PlayerActions::QUICK_TURN),
    );
    tick_with_sleep(&mut server, &mut client);

    for seq in 2..=300u32 {
        queue_intent(&mut client, seq, Vec2::ZERO, 0.0);
        tick_with_sleep(&mut server, &mut client);
    }

    let acks = drain_acks(&mut client);
    let dt = 1.0 / 30.0;

    let mut parity_checked = 0u32;
    for window in acks.windows(2) {
        let prev = &window[0];
        let next = &window[1];

        if !next
            .last_processed_sequence
            .is_after(prev.last_processed_sequence)
        {
            continue;
        }

        // Simulate one tick of snap turn from prev ack state.
        let mut sim_angle = prev.angle;
        let mut sim_remaining = prev.snap_remaining_radians;
        tick_snap_turn(
            &mut sim_angle,
            &mut sim_remaining,
            prev.snap_speed,
            prev.snap_direction,
            dt,
        );

        let angle_drift = (sim_angle - next.angle).rem_euclid(std::f32::consts::TAU);
        let angle_drift = if angle_drift > std::f32::consts::PI {
            angle_drift - std::f32::consts::TAU
        } else {
            angle_drift
        };

        assert!(
            angle_drift.abs() < 0.01,
            "snap ack parity: seq {:?} → {:?}, angle_drift={angle_drift:.4}\n\
             prev_angle={:.3} sim_angle={sim_angle:.3} next_angle={:.3}\n\
             prev_remaining={:.3} sim_remaining={sim_remaining:.3} next_remaining={:.3}",
            prev.last_processed_sequence,
            next.last_processed_sequence,
            prev.angle,
            next.angle,
            prev.snap_remaining_radians,
            next.snap_remaining_radians,
        );
        parity_checked += 1;
    }

    assert!(
        parity_checked > 0,
        "should have checked at least one consecutive ack pair ({} acks)",
        acks.len()
    );
}

/// External position mutation after `MovementSet` causes the next
/// `InputAck` to carry the mutated position via `position_diverged`.
///
/// Regression test for the lunge/occupancy ack fix: an idle player
/// whose position is changed by a server-only force (not input-driven)
/// must still receive a corrective ack.
#[test]
fn external_position_mutation_triggers_ack() {
    let port = reserve_port();
    let addr: SocketAddr = ([127, 0, 0, 1], port).into();
    let mut server = build_fixed_tick_server(port);
    let mut client = build_prediction_client(addr);
    server.update();
    assert!(wait_for_player(&mut server, &mut client));

    // Send one input to establish the sequence (ack system needs a seq).
    queue_intent(&mut client, 1, Vec2::ZERO, 0.0);
    for _ in 0..50 {
        tick_with_sleep(&mut server, &mut client);
    }
    let baseline_acks = drain_acks(&mut client);
    assert!(!baseline_acks.is_empty(), "should have baseline ack");
    let baseline_pos = baseline_acks.last().unwrap().position;

    // Externally mutate the player's position on the server (simulates
    // lunge push or occupancy separation applied after MovementSet).
    let push = Vec2::new(0.5, 0.0);
    {
        let mut q = server.world_mut().query::<&mut NetPlayer>();
        for mut player in q.iter_mut(server.world_mut()) {
            player.position += push;
        }
    }

    // Tick enough for the position_diverged check to fire and the ack
    // to reach the client over UDP.
    for _ in 0..80 {
        tick_with_sleep(&mut server, &mut client);
    }

    let correction_acks = drain_acks(&mut client);
    assert!(
        !correction_acks.is_empty(),
        "idle player with externally mutated position should receive a corrective ack"
    );

    // The corrective ack should carry the pushed position.
    let corrected_pos = correction_acks.last().unwrap().position;
    let expected_x = baseline_pos.x + push.x;
    assert!(
        (corrected_pos.x - expected_x).abs() < 0.1,
        "ack should carry pushed position: baseline={baseline_pos}, expected_x={expected_x}, got={corrected_pos}"
    );
}
