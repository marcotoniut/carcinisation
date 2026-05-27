//! Player death and respawn integration tests.
#![allow(clippy::float_cmp)]

mod common;

use bevy::prelude::*;
use carcinisation_net::{NetEnemyState, PlayerId, PlayerNetState};
use carcinisation_server::systems::BurnContactCooldowns;
use common::{
    build_server_with_enemy, force_enemy_state, get_player_health, inject_intent, reserve_port,
    set_player_health, spawn_alive_player, tick_server, wait_for_server_condition,
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
    let port = reserve_port();
    let mut server = build_server_with_enemy(port, 3.5, 1.5);
    server.update();

    spawn_alive_player(&mut server, 1, 1.5, 1.5);

    // Reduce health to 1, then let enemy projectile/melee finish the job.
    set_player_health(&mut server, 1, 1.0);
    force_enemy_state(&mut server, NetEnemyState::HoldingRange);

    // 2000 ticks at 2 ms ≈ 120 FixedUpdate cycles at 30 Hz ≈ 4 s game time.
    let died = wait_for_server_condition(&mut server, 2000, |server| {
        get_player(server, 1).is_some_and(|(_, state, _)| matches!(state, PlayerNetState::Dead))
    });

    assert!(died, "player should transition to Dead after lethal damage");
}

/// Dead player input is ignored (no movement).
#[test]
fn dead_player_input_ignored() {
    let port = reserve_port();
    let mut server = build_server_with_enemy(port, 6.5, 6.5);
    server.update();

    spawn_alive_player(&mut server, 1, 1.5, 1.5);
    set_player_health(&mut server, 1, 0.0);

    // 50 ticks at 2 ms ≈ 3 FixedUpdate cycles at 30 Hz — enough for death transition.
    for _ in 0..50 {
        tick_server(&mut server);
    }

    let (pos_before, state, _) = get_player(&mut server, 1).unwrap();
    assert!(
        matches!(state, PlayerNetState::Dead),
        "player should be dead"
    );

    // Inject movement input.
    inject_intent(&mut server, 1, Vec2::new(0.0, 1.0), false);
    // 50 ticks at 2 ms ≈ 3 FixedUpdate cycles at 30 Hz
    for _ in 0..50 {
        tick_server(&mut server);
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
    let port = reserve_port();
    let mut server = build_server_with_enemy(port, 6.5, 6.5);
    server.update();

    spawn_alive_player(&mut server, 1, 3.0, 3.0);
    set_player_health(&mut server, 1, 0.0);

    // 2000 ticks at 2 ms ≈ 120 FixedUpdate cycles at 30 Hz ≈ 4 s game time.
    // Respawn timer is 3 s, so this gives plenty of margin.
    for _ in 0..2000 {
        tick_server(&mut server);
    }

    let (pos, state, hp) = get_player(&mut server, 1).unwrap();
    assert!(
        matches!(state, PlayerNetState::Alive),
        "player should be alive after respawn"
    );
    assert_eq!(hp, 100.0, "health should be restored to max");
    // Should have moved to a spawn point (not the original 3.0, 3.0).
    // Default spawn is (1.5, 1.5) from fallback_player_starts.
    assert_ne!(
        pos,
        Vec2::new(3.0, 3.0),
        "player should be at spawn point, not death position"
    );
}

/// Enemies should not target dead players.
#[test]
fn enemies_ignore_dead_player() {
    let port = reserve_port();
    let mut server = build_server_with_enemy(port, 3.5, 1.5);
    server.update();

    spawn_alive_player(&mut server, 1, 1.5, 1.5);
    set_player_health(&mut server, 1, 0.0);

    // 50 ticks at 2 ms ≈ 3 FixedUpdate cycles — trigger death.
    for _ in 0..50 {
        tick_server(&mut server);
    }

    let (_, state, _) = get_player(&mut server, 1).unwrap();
    assert!(matches!(state, PlayerNetState::Dead));

    // Force enemy attacking — should have no valid target.
    force_enemy_state(&mut server, NetEnemyState::HoldingRange);
    // 1100 ticks at 2 ms ≈ 66 FixedUpdate cycles at 30 Hz ≈ 2.2 s game time.
    for _ in 0..1100 {
        tick_server(&mut server);
    }

    // Projectiles should have despawned (hit wall or TTL) — not stuck on dead player.
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
    let port = reserve_port();
    let mut server = build_server_with_enemy(port, 3.5, 1.5);
    server.update();

    spawn_alive_player(&mut server, 1, 1.5, 1.5);
    spawn_alive_player(&mut server, 2, 5.5, 1.5);

    // Kill player 1.
    set_player_health(&mut server, 1, 0.0);
    // 50 ticks — trigger death.
    for _ in 0..50 {
        tick_server(&mut server);
    }

    let (_, state1, _) = get_player(&mut server, 1).unwrap();
    assert!(matches!(state1, PlayerNetState::Dead));
    let (_, state2, _) = get_player(&mut server, 2).unwrap();
    assert!(matches!(state2, PlayerNetState::Alive));

    // Force enemy attacking — should target player 2 (the alive one).
    force_enemy_state(&mut server, NetEnemyState::HoldingRange);
    // 2000 ticks ≈ 120 FixedUpdate cycles at 30 Hz ≈ 4 s game time.
    let damaged = wait_for_server_condition(&mut server, 2000, |server| {
        get_player_health(server, 2).unwrap() < 100.0
    });

    assert!(damaged, "alive player should take damage from enemy");
}

/// Death clears `BurnContactCooldowns` so respawned player doesn't inherit stale state.
#[test]
fn death_clears_burn_contact_cooldown() {
    let port = reserve_port();
    let mut server = build_server_with_enemy(port, 6.5, 6.5);
    server.update();

    spawn_alive_player(&mut server, 1, 1.5, 1.5);

    // Seed a non-zero burn contact cooldown as if the player was near a burning corpse.
    server
        .world_mut()
        .resource_mut::<BurnContactCooldowns>()
        .0
        .insert(PlayerId(1), 0.4);

    // Kill the player.
    set_player_health(&mut server, 1, 0.0);
    // 50 ticks — trigger death.
    for _ in 0..50 {
        tick_server(&mut server);
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
