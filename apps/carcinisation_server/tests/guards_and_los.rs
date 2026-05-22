//! Regression tests for dead-player guards and line-of-sight blocking.
#![allow(clippy::float_cmp)]

mod common;

use bevy::prelude::*;
use bevy_replicon::prelude::*;
use carcinisation_fps_core::map::{EntitySpawnData, EntitySpawnKind, Map};
use carcinisation_net::{
    NetAttackId, NetEnemyState, NetHealth, NetPlayer, PlayerId, PlayerNetState,
};
use carcinisation_server::ServerPlugin;
use common::{build_server_app, reserve_port};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// 8x8 open map with a wall column at (4, 1) blocking east LOS from (1.5, 1.5).
fn los_test_map() -> Map {
    #[rustfmt::skip]
    let cells = vec![
        1, 1, 1, 1, 1, 1, 1, 1,
        1, 0, 0, 0, 1, 0, 0, 1,  // wall at (4,1) blocks (1.5,1.5) → (5.5,1.5)
        1, 0, 0, 0, 0, 0, 0, 1,
        1, 0, 0, 0, 0, 0, 0, 1,
        1, 0, 0, 0, 0, 0, 0, 1,
        1, 0, 0, 0, 0, 0, 0, 1,
        1, 0, 0, 0, 0, 0, 0, 1,
        1, 1, 1, 1, 1, 1, 1, 1,
    ];
    Map {
        width: 8,
        height: 8,
        cells,
    }
}

fn build_server_with_map_and_enemy(port: u16, map: Map, enemy_x: f32, enemy_y: f32) -> App {
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
        map,
        entities,
        player_starts: vec![],
        admin_socket: None,
        instance_name: "test".to_string(),
        map_path: "test_map".to_string(),
    })
}

fn spawn_player(server: &mut App, player_id: u32, x: f32, y: f32, state: PlayerNetState) {
    let hp = if matches!(&state, PlayerNetState::Alive) {
        100.0
    } else {
        0.0
    };
    server.world_mut().spawn((
        NetPlayer {
            player_id: PlayerId(player_id),
            position: Vec2::new(x, y),
            angle: 0.0,
            current_attack: NetAttackId::None,
            state,
            flame_active: false,
            avatar_palette_variant: None,
        },
        NetHealth {
            current: hp,
            max: 100.0,
        },
        Replicated,
    ));
}

fn spawn_dead_player(server: &mut App, player_id: u32, x: f32, y: f32) {
    spawn_player(server, player_id, x, y, PlayerNetState::Dead);
}

fn tick_server(server: &mut App) {
    std::thread::sleep(std::time::Duration::from_millis(2));
    server.update();
}

fn get_player_position(server: &mut App, pid: u32) -> Option<Vec2> {
    server
        .world_mut()
        .query::<&NetPlayer>()
        .iter(server.world())
        .find(|p| p.player_id.0 == pid)
        .map(|p| p.position)
}

fn get_enemy_health(server: &mut App) -> Option<f32> {
    server
        .world_mut()
        .query::<(&carcinisation_net::components::NetEnemy, &NetHealth)>()
        .iter(server.world())
        .next()
        .map(|(_, h)| h.current)
}

fn inject_intent(server: &mut App, pid: u32, movement: bevy::math::Vec2, fire_held: bool) {
    use carcinisation_net::{ClientIntent, InputSequence, PlayerActions};
    use carcinisation_server::systems::PlayerIntentBuffer;
    server.world_mut().resource_mut::<PlayerIntentBuffer>().set(
        PlayerId(pid),
        &ClientIntent {
            sequence: InputSequence(0),
            movement,
            turn: 0.0,
            fire_held,
            actions: PlayerActions::default(),
        },
    );
}

fn force_player_attack(server: &mut App, pid: u32, attack: NetAttackId) {
    let mut q = server.world_mut().query::<&mut NetPlayer>();
    for mut p in q.iter_mut(server.world_mut()) {
        if p.player_id.0 == pid {
            p.current_attack = attack;
        }
    }
}

// ---------------------------------------------------------------------------
// Dead-player guard tests
// ---------------------------------------------------------------------------

/// Dead player input does not cause movement.
#[test]
fn dead_player_cannot_move() {
    let port = reserve_port();
    let mut server =
        build_server_with_map_and_enemy(port, carcinisation_fps_core::map::test_map(), 6.5, 6.5);
    server.update();

    spawn_dead_player(&mut server, 1, 1.5, 1.5);

    let pos_before = get_player_position(&mut server, 1).unwrap();

    // Inject forward input for the dead player.
    inject_intent(&mut server, 1, bevy::math::Vec2::new(0.0, 1.0), false);
    for _ in 0..50 {
        tick_server(&mut server);
    }

    let pos_after = get_player_position(&mut server, 1).unwrap();
    assert_eq!(
        pos_before, pos_after,
        "dead player should not move: {pos_before} → {pos_after}"
    );
}

/// Dead player fire input does not deal hitscan damage.
#[test]
fn dead_player_cannot_fire_pistol() {
    let port = reserve_port();
    // Enemy at (4.5, 1.5), directly east of player spawn.
    let mut server =
        build_server_with_map_and_enemy(port, carcinisation_fps_core::map::test_map(), 4.5, 1.5);
    server.update();

    spawn_dead_player(&mut server, 1, 1.5, 1.5);
    let hp_before = get_enemy_health(&mut server).unwrap();

    // Inject fire input.
    inject_intent(&mut server, 1, bevy::math::Vec2::ZERO, true);
    for _ in 0..50 {
        tick_server(&mut server);
    }

    let hp_after = get_enemy_health(&mut server).unwrap();
    assert_eq!(
        hp_before, hp_after,
        "dead player pistol should not damage enemy: {hp_before} → {hp_after}"
    );
}

/// Dead player with flamethrower does not deal damage.
#[test]
fn dead_player_cannot_use_flamethrower() {
    let port = reserve_port();
    // Enemy at (3.5, 1.5), within flame range.
    let mut server =
        build_server_with_map_and_enemy(port, carcinisation_fps_core::map::test_map(), 3.5, 1.5);
    server.update();

    spawn_dead_player(&mut server, 1, 1.5, 1.5);
    force_player_attack(&mut server, 1, NetAttackId::Projectile);
    let hp_before = get_enemy_health(&mut server).unwrap();

    // Inject fire input.
    inject_intent(&mut server, 1, bevy::math::Vec2::ZERO, true);
    for _ in 0..50 {
        tick_server(&mut server);
    }

    let hp_after = get_enemy_health(&mut server).unwrap();
    assert_eq!(
        hp_before, hp_after,
        "dead player flamethrower should not damage enemy: {hp_before} → {hp_after}"
    );
}

// ---------------------------------------------------------------------------
// LOS blocking tests
// ---------------------------------------------------------------------------

/// Pistol hitscan blocked by wall does not damage enemy behind it.
#[test]
fn pistol_blocked_by_wall() {
    let port = reserve_port();
    // Player at (1.5, 1.5), enemy at (5.5, 1.5), wall at (4, 1) between them.
    let mut server = build_server_with_map_and_enemy(port, los_test_map(), 5.5, 1.5);
    server.update();

    spawn_player(&mut server, 1, 1.5, 1.5, PlayerNetState::Alive);
    let hp_before = get_enemy_health(&mut server).unwrap();

    inject_intent(&mut server, 1, bevy::math::Vec2::ZERO, true);
    for _ in 0..50 {
        tick_server(&mut server);
    }

    let hp_after = get_enemy_health(&mut server).unwrap();
    assert_eq!(
        hp_before, hp_after,
        "pistol should not damage enemy behind wall: {hp_before} → {hp_after}"
    );
}

/// Flamethrower blocked by wall does not damage enemy behind it.
#[test]
fn flamethrower_blocked_by_wall() {
    let port = reserve_port();
    // Player at (1.5, 1.5), enemy at (5.5, 1.5), wall at (4, 1) between them.
    let mut server = build_server_with_map_and_enemy(port, los_test_map(), 5.5, 1.5);
    server.update();

    spawn_player(&mut server, 1, 1.5, 1.5, PlayerNetState::Alive);
    force_player_attack(&mut server, 1, NetAttackId::Projectile);
    let hp_before = get_enemy_health(&mut server).unwrap();

    inject_intent(&mut server, 1, bevy::math::Vec2::ZERO, true);
    for _ in 0..50 {
        tick_server(&mut server);
    }

    let hp_after = get_enemy_health(&mut server).unwrap();
    assert_eq!(
        hp_before, hp_after,
        "flamethrower should not damage enemy behind wall: {hp_before} → {hp_after}"
    );
}

/// Enemy projectile hits wall and despawns before reaching player behind it.
#[test]
fn enemy_projectile_blocked_by_wall() {
    let port = reserve_port();
    // Enemy at (5.5, 1.5), player at (1.5, 1.5), wall at (4, 1) between them.
    let mut server = build_server_with_map_and_enemy(port, los_test_map(), 5.5, 1.5);
    server.update();

    spawn_player(&mut server, 1, 1.5, 1.5, PlayerNetState::Alive);

    // Force enemy into attacking state.
    {
        let mut q = server
            .world_mut()
            .query::<&mut carcinisation_net::components::NetEnemy>();
        for mut e in q.iter_mut(server.world_mut()) {
            e.state = NetEnemyState::HoldingRange;
        }
    }

    // Wait for projectile spawn + travel.
    for _ in 0..2000 {
        tick_server(&mut server);
    }

    // Player health should be untouched (projectile hit wall).
    let hp = server
        .world_mut()
        .query::<(&NetPlayer, &NetHealth)>()
        .iter(server.world())
        .next()
        .map(|(_, h)| h.current)
        .unwrap();
    assert_eq!(
        hp, 100.0,
        "player behind wall should not take projectile damage: hp={hp}"
    );
}
