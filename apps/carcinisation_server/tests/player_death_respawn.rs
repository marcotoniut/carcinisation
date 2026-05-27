//! Player death and respawn integration tests.
//!
//! Deterministic: each `app.update()` = exactly one FixedUpdate cycle at 30 Hz.
#![allow(clippy::float_cmp)]

mod common;

use bevy::prelude::*;
use carcinisation_net::{NetEnemyState, PlayerId, PlayerNetState};
use carcinisation_server::systems::BurnContactCooldowns;
use common::{
    build_deterministic_server_with_enemy, force_enemy_state, get_player_health, inject_intent,
    set_player_health, spawn_alive_player, wait_for_deterministic,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn get_player(server: &mut App, pid: u32) -> Option<(Vec2, PlayerNetState, f32)> {
    server
        .world_mut()
        .query::<(&carcinisation_net::NetPlayer, &carcinisation_net::NetHealth)>()
        .iter(server.world())
        .find(|(p, _)| p.player_id.0 == pid)
        .map(|(p, h)| (p.position, p.state.clone(), h.current))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Lethal projectile damage transitions player to Dead state.
#[test]
fn lethal_damage_sets_dead() {
    let mut server = build_deterministic_server_with_enemy(3.5, 1.5);
    server.update();

    spawn_alive_player(&mut server, 1, 1.5, 1.5);

    // Reduce health to 1, then let enemy projectile/melee finish the job.
    set_player_health(&mut server, 1, 1.0);
    force_enemy_state(&mut server, NetEnemyState::HoldingRange);

    // 120 fixed ticks at 30 Hz = 4 s — enough for projectile spawn + hit.
    let died = wait_for_deterministic(&mut server, 120, |server| {
        get_player(server, 1).is_some_and(|(_, state, _)| matches!(state, PlayerNetState::Dead))
    });

    assert!(died, "player should transition to Dead after lethal damage");
}

/// Dead player input is ignored (no movement).
#[test]
fn dead_player_input_ignored() {
    let mut server = build_deterministic_server_with_enemy(6.5, 6.5);
    server.update();

    spawn_alive_player(&mut server, 1, 1.5, 1.5);
    set_player_health(&mut server, 1, 0.0);

    // 3 fixed ticks — trigger death transition.
    for _ in 0..3 {
        server.update();
    }

    let (pos_before, state, _) = get_player(&mut server, 1).unwrap();
    assert!(
        matches!(state, PlayerNetState::Dead),
        "player should be dead"
    );

    // Inject movement input.
    inject_intent(&mut server, 1, Vec2::new(0.0, 1.0), false);
    // 3 fixed ticks at 30 Hz = 0.1 s
    for _ in 0..3 {
        server.update();
    }

    let (pos_after, _, _) = get_player(&mut server, 1).unwrap();
    assert_eq!(
        pos_before, pos_after,
        "dead player should not move: {pos_before} → {pos_after}"
    );
}

/// After respawn timer expires, player is alive with full health at a spawn point.
#[test]
fn respawn_restores_health_and_position() {
    let mut server = build_deterministic_server_with_enemy(6.5, 6.5);
    server.update();

    spawn_alive_player(&mut server, 1, 3.0, 3.0);
    set_player_health(&mut server, 1, 0.0);

    // Respawn timer = 3 s = 90 fixed ticks at 30 Hz. Wait up to 120.
    for _ in 0..120 {
        server.update();
    }

    let (pos, state, hp) = get_player(&mut server, 1).unwrap();
    assert!(
        matches!(state, PlayerNetState::Alive),
        "player should be alive after respawn"
    );
    assert_eq!(hp, 100.0, "health should be restored to max");
    // Should have moved to a spawn point (not the original 3.0, 3.0).
    assert_ne!(
        pos,
        Vec2::new(3.0, 3.0),
        "player should be at spawn point, not death position"
    );
}

/// Enemies should not target dead players.
#[test]
fn enemies_ignore_dead_player() {
    let mut server = build_deterministic_server_with_enemy(3.5, 1.5);
    server.update();

    spawn_alive_player(&mut server, 1, 1.5, 1.5);
    set_player_health(&mut server, 1, 0.0);

    // 3 fixed ticks — trigger death.
    for _ in 0..3 {
        server.update();
    }

    let (_, state, _) = get_player(&mut server, 1).unwrap();
    assert!(matches!(state, PlayerNetState::Dead));

    // Force enemy attacking — should have no valid target.
    force_enemy_state(&mut server, NetEnemyState::HoldingRange);
    // 66 fixed ticks at 30 Hz = 2.2 s
    for _ in 0..66 {
        server.update();
    }

    // Projectiles should have despawned (hit wall or TTL).
    let proj_count = server
        .world_mut()
        .query::<&carcinisation_net::NetProjectile>()
        .iter(server.world())
        .count();
    let (_, _, hp) = get_player(&mut server, 1).unwrap();
    assert!(
        proj_count == 0 || hp > 0.0,
        "enemy should not damage dead player; proj_count={proj_count} hp={hp}"
    );
}

/// With two players, killing one leaves the other targetable.
#[test]
fn alive_player_remains_targetable_when_other_dies() {
    let mut server = build_deterministic_server_with_enemy(3.5, 1.5);
    server.update();

    spawn_alive_player(&mut server, 1, 1.5, 1.5);
    spawn_alive_player(&mut server, 2, 5.5, 1.5);

    // Kill player 1.
    set_player_health(&mut server, 1, 0.0);
    // 3 fixed ticks — trigger death.
    for _ in 0..3 {
        server.update();
    }

    let (_, state1, _) = get_player(&mut server, 1).unwrap();
    assert!(matches!(state1, PlayerNetState::Dead));
    let (_, state2, _) = get_player(&mut server, 2).unwrap();
    assert!(matches!(state2, PlayerNetState::Alive));

    // Force enemy attacking — should target player 2.
    force_enemy_state(&mut server, NetEnemyState::HoldingRange);
    // 120 fixed ticks at 30 Hz = 4 s
    let damaged = wait_for_deterministic(&mut server, 120, |server| {
        get_player_health(server, 2).unwrap() < 100.0
    });

    assert!(damaged, "alive player should take damage from enemy");
}

/// Death clears `BurnContactCooldowns` so respawned player doesn't inherit stale state.
#[test]
fn death_clears_burn_contact_cooldown() {
    let mut server = build_deterministic_server_with_enemy(6.5, 6.5);
    server.update();

    spawn_alive_player(&mut server, 1, 1.5, 1.5);

    // Seed a non-zero burn contact cooldown.
    server
        .world_mut()
        .resource_mut::<BurnContactCooldowns>()
        .0
        .insert(PlayerId(1), 0.4);

    // Kill the player.
    set_player_health(&mut server, 1, 0.0);
    // 3 fixed ticks — trigger death.
    for _ in 0..3 {
        server.update();
    }

    let (_, state, _) = get_player(&mut server, 1).unwrap();
    assert!(
        matches!(state, PlayerNetState::Dead),
        "player should be dead"
    );
    assert!(
        !server
            .world()
            .resource::<BurnContactCooldowns>()
            .0
            .contains_key(&PlayerId(1)),
        "BurnContactCooldowns should be cleared on death"
    );
}
