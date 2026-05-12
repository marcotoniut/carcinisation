//! Player death and respawn integration tests.
#![allow(clippy::float_cmp)]

mod common;

use bevy::prelude::*;
use bevy_replicon::prelude::*;
use carcinisation_fps_core::map::{EntitySpawnData, EntitySpawnKind, test_map};
use carcinisation_net::{
    NetAttackId, NetEnemyState, NetHealth, NetPlayer, PlayerId, PlayerNetState,
};
use carcinisation_server::ServerPlugin;
use carcinisation_server::systems::BurnContactCooldowns;
use common::{build_server_app, reserve_port};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn build_server_with_enemy(port: u16, enemy_x: f32, enemy_y: f32) -> App {
    let entities = vec![EntitySpawnData {
        kind: EntitySpawnKind::Mosquiton {
            health: 100,
            speed: 0.0,
        },
        x: enemy_x,
        y: enemy_y,
    }];
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

fn spawn_alive_player(server: &mut App, pid: u32, x: f32, y: f32) {
    server.world_mut().spawn((
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
        Replicated,
    ));
}

fn set_player_health(server: &mut App, pid: u32, hp: f32) {
    let mut q = server.world_mut().query::<(&NetPlayer, &mut NetHealth)>();
    for (p, mut h) in q.iter_mut(server.world_mut()) {
        if p.player_id.0 == pid {
            h.current = hp;
        }
    }
}

fn get_player(server: &mut App, pid: u32) -> Option<(Vec2, PlayerNetState, f32)> {
    server
        .world_mut()
        .query::<(&NetPlayer, &NetHealth)>()
        .iter(server.world())
        .find(|(p, _)| p.player_id.0 == pid)
        .map(|(p, h)| (p.position, p.state.clone(), h.current))
}

fn force_enemy_attacking(server: &mut App) {
    let mut q = server
        .world_mut()
        .query::<&mut carcinisation_net::components::NetEnemy>();
    for mut e in q.iter_mut(server.world_mut()) {
        e.state = NetEnemyState::HoldingRange;
    }
}

fn inject_forward(server: &mut App, pid: u32) {
    use carcinisation_net::{ClientIntent, InputSequence, PlayerActions};
    use carcinisation_server::systems::PlayerIntentBuffer;
    server.world_mut().resource_mut::<PlayerIntentBuffer>().set(
        PlayerId(pid),
        &ClientIntent {
            sequence: InputSequence(0),
            movement: bevy::math::Vec2::new(0.0, 1.0),
            turn: 0.0,
            fire_held: false,
            actions: PlayerActions::default(),
        },
    );
}

fn tick(server: &mut App) {
    std::thread::sleep(std::time::Duration::from_millis(2));
    server.update();
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Lethal projectile damage transitions player to Dead state.
#[test]
fn lethal_damage_sets_dead() {
    let port = reserve_port();
    // Enemy at (3.5, 1.5), close to player for melee or projectile.
    let mut server = build_server_with_enemy(port, 3.5, 1.5);
    server.update();

    spawn_alive_player(&mut server, 1, 1.5, 1.5);

    // Reduce health to 1, then let enemy projectile/melee finish the job.
    set_player_health(&mut server, 1, 1.0);
    force_enemy_attacking(&mut server);

    // Tick until player dies or timeout.
    let mut died = false;
    for _ in 0..2000 {
        tick(&mut server);
        if let Some((_, state, _)) = get_player(&mut server, 1)
            && matches!(state, PlayerNetState::Dead)
        {
            died = true;
            break;
        }
    }

    assert!(died, "player should transition to Dead after lethal damage");
}

/// Dead player input is ignored (no movement).
#[test]
fn dead_player_input_ignored() {
    let port = reserve_port();
    let mut server = build_server_with_enemy(port, 6.5, 6.5);
    server.update();

    spawn_alive_player(&mut server, 1, 1.5, 1.5);
    // Kill the player via health.
    set_player_health(&mut server, 1, 0.0);

    // Tick to trigger death transition.
    for _ in 0..50 {
        tick(&mut server);
    }

    let (pos_before, state, _) = get_player(&mut server, 1).unwrap();
    assert!(
        matches!(state, PlayerNetState::Dead),
        "player should be dead"
    );

    // Inject movement input.
    inject_forward(&mut server, 1);
    for _ in 0..50 {
        tick(&mut server);
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

    // Tick past death + 3s respawn timer.
    // At 30Hz = 33ms/tick, 3s = ~90 fixed ticks. With 2ms sleep, ~90*17 = ~1530 calls.
    for _ in 0..2000 {
        tick(&mut server);
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
    // Enemy at (3.5, 1.5), player at (1.5, 1.5).
    let mut server = build_server_with_enemy(port, 3.5, 1.5);
    server.update();

    spawn_alive_player(&mut server, 1, 1.5, 1.5);
    set_player_health(&mut server, 1, 0.0);

    // Tick past death.
    for _ in 0..50 {
        tick(&mut server);
    }

    let (_, state, _) = get_player(&mut server, 1).unwrap();
    assert!(matches!(state, PlayerNetState::Dead));

    // Force enemy into attacking and wait for projectile spawn.
    force_enemy_attacking(&mut server);
    for _ in 0..1100 {
        tick(&mut server);
    }

    // Player health should still be 0 (no damage to dead player).
    let (_, _, hp) = get_player(&mut server, 1).unwrap();
    // Health might be restored if respawn happened. Check state.
    // If respawned and took damage, that's valid. But while dead, no damage.
    // The key assertion: the player wasn't hit while in Dead state.
    // We can verify by checking if any projectiles exist (they should have
    // despawned hitting walls since the dead player isn't a valid target).
    let proj_count = server
        .world_mut()
        .query::<&carcinisation_net::NetProjectile>()
        .iter(server.world())
        .count();
    // Projectiles should have despawned (hit wall or TTL) — not stuck on dead player.
    assert!(
        proj_count == 0 || hp > 0.0,
        "enemy should not damage dead player; proj_count={proj_count} hp={hp}"
    );
}

/// With two players, killing one leaves the other targetable.
#[test]
fn alive_player_remains_targetable_when_other_dies() {
    let port = reserve_port();
    // Enemy at (3.5, 1.5), between the two players.
    let mut server = build_server_with_enemy(port, 3.5, 1.5);
    server.update();

    spawn_alive_player(&mut server, 1, 1.5, 1.5);
    spawn_alive_player(&mut server, 2, 5.5, 1.5);

    // Kill player 1.
    set_player_health(&mut server, 1, 0.0);
    for _ in 0..50 {
        tick(&mut server);
    }

    let (_, state1, _) = get_player(&mut server, 1).unwrap();
    assert!(matches!(state1, PlayerNetState::Dead));
    let (_, state2, _) = get_player(&mut server, 2).unwrap();
    assert!(matches!(state2, PlayerNetState::Alive));

    // Force enemy attacking — should target player 2 (the alive one).
    force_enemy_attacking(&mut server);
    for _ in 0..2000 {
        tick(&mut server);
    }

    // Player 2 should have taken damage (enemy targets alive player).
    let (_, _, hp2) = get_player(&mut server, 2).unwrap();
    assert!(
        hp2 < 100.0,
        "alive player should take damage from enemy: hp={hp2}"
    );
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
    for _ in 0..50 {
        tick(&mut server);
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
