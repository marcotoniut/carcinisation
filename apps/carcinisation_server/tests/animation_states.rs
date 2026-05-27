//! Tests for enemy animation-state semantics.
#![allow(clippy::doc_markdown)]

mod common;

use carcinisation_net::{NetAttackId, NetEnemyState};
use common::{
    build_server_with_enemy, force_player_attack, get_enemy_state, inject_fire, reserve_port,
    set_enemy_health, spawn_alive_player, tick_server, wait_for_server_condition,
};

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Enemy at preferred range maps to HoldingRange, not the old Attack.
#[test]
fn enemy_at_range_uses_holding_state() {
    let port = reserve_port();
    // Enemy at (4.5, 1.5), player at (1.5, 1.5). Distance = 3.0 ≈ preferred_range.
    let mut server = build_server_with_enemy(port, 4.5, 1.5);
    server.update();

    spawn_alive_player(&mut server, 1, 1.5, 1.5);

    // 100 ticks at 2 ms ≈ 6 FixedUpdate cycles at 30 Hz — AI evaluates.
    for _ in 0..100 {
        tick_server(&mut server);
    }

    let state = get_enemy_state(&mut server).unwrap();
    assert_eq!(
        state,
        NetEnemyState::HoldingRange,
        "enemy at preferred range should be HoldingRange, got {state:?}"
    );
}

/// Lethal hitscan (non-burn) damage transitions enemy to Dying { burn: false }.
#[test]
fn hitscan_kill_transitions_to_dying_not_dead() {
    let port = reserve_port();
    // Enemy at (3.0, 1.5), directly east of player at (1.5, 1.5).
    let mut server = build_server_with_enemy(port, 3.0, 1.5);
    server.update();

    spawn_alive_player(&mut server, 1, 1.5, 1.5);
    set_enemy_health(&mut server, 1.0);

    inject_fire(&mut server, 1);
    // 50 ticks at 2 ms ≈ 3 FixedUpdate cycles at 30 Hz
    for _ in 0..50 {
        tick_server(&mut server);
    }

    let state = get_enemy_state(&mut server).unwrap();
    assert!(
        matches!(
            state,
            NetEnemyState::Dying { burn: false } | NetEnemyState::Dead { burn: false }
        ),
        "hitscan kill should produce Dying/Dead {{ burn: false }}, got {state:?}"
    );
}

/// Lethal flamethrower damage transitions enemy to Dying { burn: true }.
#[test]
fn flamethrower_kill_transitions_to_dying_burn() {
    let port = reserve_port();
    // Enemy at (3.0, 1.5), within flame range.
    let mut server = build_server_with_enemy(port, 3.0, 1.5);
    server.update();

    spawn_alive_player(&mut server, 1, 1.5, 1.5);
    force_player_attack(&mut server, 1, NetAttackId::Projectile);
    set_enemy_health(&mut server, 1.0);

    // Fire flamethrower — burn system builds intensity progressively.
    inject_fire(&mut server, 1);
    // 300 ticks at 2 ms ≈ 18 FixedUpdate cycles at 30 Hz ≈ 0.6 s game time.
    for _ in 0..300 {
        tick_server(&mut server);
    }

    let state = get_enemy_state(&mut server).unwrap();
    assert!(
        matches!(
            state,
            NetEnemyState::Dying { burn: true } | NetEnemyState::Dead { burn: true }
        ),
        "flamethrower kill should produce Dying {{ burn: true }}, got {state:?}"
    );
}

/// Dying enemy transitions to Dead after death timer expires.
#[test]
fn dying_transitions_to_dead_after_timer() {
    let port = reserve_port();
    let mut server = build_server_with_enemy(port, 3.0, 1.5);
    server.update();

    spawn_alive_player(&mut server, 1, 1.5, 1.5);
    set_enemy_health(&mut server, 1.0);

    inject_fire(&mut server, 1);
    // 50 ticks — trigger the kill.
    for _ in 0..50 {
        tick_server(&mut server);
    }

    let state = get_enemy_state(&mut server).unwrap();
    assert!(
        matches!(state, NetEnemyState::Dying { .. }),
        "should be Dying after lethal damage, got {state:?}"
    );

    // Wait for death timer (0.5 s). 400 ticks at 2 ms ≈ 24 FixedUpdate cycles at 30 Hz ≈ 0.8 s.
    let transitioned = wait_for_server_condition(&mut server, 400, |server| {
        matches!(
            get_enemy_state(server),
            Some(NetEnemyState::Dead { burn: false })
        )
    });

    assert!(
        transitioned,
        "should transition to Dead {{ burn: false }} after pistol death timer"
    );
}
