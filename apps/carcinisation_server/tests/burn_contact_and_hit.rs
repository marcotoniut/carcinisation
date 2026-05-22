//! Tests for burning corpse contact damage and hitscan hit confirmation.
#![allow(clippy::doc_markdown, clippy::float_cmp)]

mod common;

use bevy::prelude::*;
use bevy_replicon::prelude::*;
use carcinisation_fps_core::map::{EntitySpawnData, EntitySpawnKind, test_map};
use carcinisation_net::{
    NetAttackId, NetEnemyState, NetHealth, NetPlayer, PlayerId, PlayerNetState,
};
use carcinisation_server::ServerPlugin;
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
            avatar_palette_variant: None,
        },
        NetHealth {
            current: 100.0,
            max: 100.0,
        },
        Replicated,
    ));
}

fn get_player_health(server: &mut App, pid: u32) -> Option<f32> {
    server
        .world_mut()
        .query::<(&NetPlayer, &NetHealth)>()
        .iter(server.world())
        .find(|(p, _)| p.player_id.0 == pid)
        .map(|(_, h)| h.current)
}

fn inject_fire(server: &mut App, pid: u32) {
    use carcinisation_net::{ClientIntent, InputSequence, PlayerActions};
    use carcinisation_server::systems::PlayerIntentBuffer;
    server.world_mut().resource_mut::<PlayerIntentBuffer>().set(
        PlayerId(pid),
        &ClientIntent {
            sequence: InputSequence(0),
            movement: bevy::math::Vec2::ZERO,
            turn: 0.0,
            fire_held: true,
            actions: PlayerActions::default(),
        },
    );
}

fn tick(server: &mut App) {
    std::thread::sleep(std::time::Duration::from_millis(2));
    server.update();
}

// ---------------------------------------------------------------------------
// Burning corpse contact damage tests
// ---------------------------------------------------------------------------

/// Player near a burning corpse takes contact damage over time.
#[test]
fn burn_corpse_damages_nearby_player() {
    let port = reserve_port();
    // Enemy at (3.0, 1.5), player at (3.2, 1.5) — very close.
    let mut server = build_server_with_enemy(port, 3.0, 1.5);
    server.update();

    spawn_alive_player(&mut server, 1, 3.2, 1.5);

    // Kill enemy with flamethrower to create burning corpse.
    {
        let mut q = server
            .world_mut()
            .query::<(&mut carcinisation_net::components::NetEnemy, &mut NetHealth)>();
        for (mut e, mut h) in q.iter_mut(server.world_mut()) {
            e.state = NetEnemyState::Dying { burn: true };
            h.current = 0.0;
        }
    }

    let hp_before = get_player_health(&mut server, 1).unwrap();

    // Tick for burn contact to apply.
    for _ in 0..500 {
        tick(&mut server);
    }

    let hp_after = get_player_health(&mut server, 1).unwrap();
    assert!(
        hp_after < hp_before,
        "player near burning corpse should take contact damage: {hp_before} → {hp_after}"
    );
}

/// Player far from burning corpse takes no contact damage.
#[test]
fn burn_corpse_does_not_damage_distant_player() {
    let port = reserve_port();
    // Enemy at (1.5, 1.5), player at (6.5, 6.5) — far away.
    let mut server = build_server_with_enemy(port, 1.5, 1.5);
    server.update();

    spawn_alive_player(&mut server, 1, 6.5, 6.5);

    // Create burning corpse.
    {
        let mut q = server
            .world_mut()
            .query::<(&mut carcinisation_net::components::NetEnemy, &mut NetHealth)>();
        for (mut e, mut h) in q.iter_mut(server.world_mut()) {
            e.state = NetEnemyState::Dying { burn: true };
            h.current = 0.0;
        }
    }

    for _ in 0..500 {
        tick(&mut server);
    }

    let hp = get_player_health(&mut server, 1).unwrap();
    assert_eq!(
        hp, 100.0,
        "distant player should not take burn contact damage"
    );
}

// ---------------------------------------------------------------------------
// Pending projectile / shoot lead tests
// ---------------------------------------------------------------------------

/// Pending projectile system spawns NetProjectile after delay.
#[test]
fn pending_projectile_spawns_after_delay() {
    let port = reserve_port();
    let mut server = build_server_with_enemy(port, 3.5, 1.5);
    server.update();

    spawn_alive_player(&mut server, 1, 1.5, 1.5);

    // Force enemy to attacking range.
    {
        let mut q = server
            .world_mut()
            .query::<&mut carcinisation_net::components::NetEnemy>();
        for mut e in q.iter_mut(server.world_mut()) {
            e.state = NetEnemyState::HoldingRange;
        }
    }

    // Tick until projectile spawns (cooldown 2s + lead 0.1s).
    let mut found = false;
    for _ in 0..2000 {
        tick(&mut server);
        let count = server
            .world_mut()
            .query::<&carcinisation_net::NetProjectile>()
            .iter(server.world())
            .count();
        if count > 0 {
            found = true;
            break;
        }
    }

    assert!(found, "pending projectile should eventually spawn");
}

// ---------------------------------------------------------------------------
// Hitscan hit confirmation test
// ---------------------------------------------------------------------------

/// Hitscan hit on enemy produces damage (existing test validates this,
/// but we verify the enemy takes exactly HITSCAN_DAMAGE = 37).
#[test]
fn hitscan_hit_damages_enemy() {
    let port = reserve_port();
    // Enemy at (3.0, 1.5), player at (1.5, 1.5) facing east.
    let mut server = build_server_with_enemy(port, 3.0, 1.5);
    server.update();

    spawn_alive_player(&mut server, 1, 1.5, 1.5);

    let hp_before = server
        .world_mut()
        .query::<(&carcinisation_net::components::NetEnemy, &NetHealth)>()
        .iter(server.world())
        .next()
        .map(|(_, h)| h.current)
        .unwrap();

    inject_fire(&mut server, 1);
    for _ in 0..50 {
        tick(&mut server);
    }

    let hp_after = server
        .world_mut()
        .query::<(&carcinisation_net::components::NetEnemy, &NetHealth)>()
        .iter(server.world())
        .next()
        .map(|(_, h)| h.current)
        .unwrap();

    let damage = hp_before - hp_after;
    assert!(
        (damage - carcinisation_fps_core::FpsCombatConfig::default().hitscan_damage).abs() < 1.0,
        "hitscan should deal {} damage: got {damage}",
        carcinisation_fps_core::FpsCombatConfig::default().hitscan_damage
    );
}

// ---------------------------------------------------------------------------
// Projectile-player collision test
// ---------------------------------------------------------------------------

/// Enemy projectile hitting a player reduces their health.
#[test]
fn enemy_projectile_damages_player() {
    use carcinisation_net::{NetProjectile, NetProjectileType, NetworkObjectId, Owner};

    let port = reserve_port();
    let mut server = build_server_with_enemy(port, 6.5, 6.5);
    server.update();

    spawn_alive_player(&mut server, 1, 3.0, 1.5);

    // Spawn a projectile aimed directly at the player from the east.
    server.world_mut().spawn((
        NetProjectile {
            object_id: NetworkObjectId(99),
            position: bevy::math::Vec2::new(5.0, 1.5),
            angle: std::f32::consts::PI, // Facing west toward player at (3.0, 1.5).
            owner: Owner(PlayerId(0)),
            damage: 15.0,
            projectile_type: NetProjectileType::BloodShot,
        },
        carcinisation_server::systems::ProjectileTtl(3.0),
        bevy_replicon::prelude::Replicated,
    ));

    let hp_before = get_player_health(&mut server, 1).unwrap();

    // Tick until projectile reaches the player (2 units at speed 4 = 0.5s).
    for _ in 0..500 {
        tick(&mut server);
    }

    let hp_after = get_player_health(&mut server, 1).unwrap();
    assert!(
        hp_after < hp_before,
        "enemy projectile should damage player: {hp_before} → {hp_after}"
    );
}
