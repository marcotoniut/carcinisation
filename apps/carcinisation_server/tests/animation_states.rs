//! Tests for enemy animation-state semantics.
//!
//! Deterministic: each `app.update()` = exactly one FixedUpdate cycle at 30 Hz.
#![allow(clippy::doc_markdown)]

mod common;

use carcinisation_net::{NetAttackId, NetEnemyState};
use common::{
    build_deterministic_server_with_enemy, force_player_attack, get_enemy_state, inject_fire,
    set_enemy_health, spawn_alive_player, wait_for_deterministic,
};

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Enemy at preferred range maps to HoldingRange, not the old Attack.
#[test]
fn enemy_at_range_uses_holding_state() {
    // Enemy at (4.5, 1.5), player at (1.5, 1.5). Distance = 3.0 ≈ preferred_range.
    let mut server = build_deterministic_server_with_enemy(4.5, 1.5);
    server.update();

    spawn_alive_player(&mut server, 1, 1.5, 1.5);

    // 6 fixed ticks at 30 Hz = 0.2 s — AI evaluates.
    for _ in 0..6 {
        server.update();
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
    // Enemy at (3.0, 1.5), directly east of player at (1.5, 1.5).
    let mut server = build_deterministic_server_with_enemy(3.0, 1.5);
    server.update();

    spawn_alive_player(&mut server, 1, 1.5, 1.5);
    set_enemy_health(&mut server, 1.0);

    inject_fire(&mut server, 1);
    // 3 fixed ticks at 30 Hz = 0.1 s
    for _ in 0..3 {
        server.update();
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
    // Enemy at (3.0, 1.5), within flame range.
    let mut server = build_deterministic_server_with_enemy(3.0, 1.5);
    server.update();

    spawn_alive_player(&mut server, 1, 1.5, 1.5);
    force_player_attack(&mut server, 1, NetAttackId::Projectile);
    set_enemy_health(&mut server, 1.0);

    // Fire flamethrower — burn system builds intensity progressively.
    inject_fire(&mut server, 1);
    // 18 fixed ticks at 30 Hz = 0.6 s
    for _ in 0..18 {
        server.update();
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
    let mut server = build_deterministic_server_with_enemy(3.0, 1.5);
    server.update();

    spawn_alive_player(&mut server, 1, 1.5, 1.5);
    set_enemy_health(&mut server, 1.0);

    inject_fire(&mut server, 1);
    // 3 fixed ticks — trigger the kill.
    for _ in 0..3 {
        server.update();
    }

    let state = get_enemy_state(&mut server).unwrap();
    assert!(
        matches!(state, NetEnemyState::Dying { .. }),
        "should be Dying after lethal damage, got {state:?}"
    );

    // Death timer = 0.5 s = 15 fixed ticks at 30 Hz. Wait up to 20.
    let transitioned = wait_for_deterministic(&mut server, 20, |server| {
        matches!(
            common::get_enemy_state(server),
            Some(NetEnemyState::Dead { burn: false })
        )
    });

    assert!(
        transitioned,
        "should transition to Dead {{ burn: false }} after pistol death timer"
    );
}
