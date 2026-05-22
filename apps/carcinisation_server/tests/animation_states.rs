//! Tests for enemy animation-state semantics.
#![allow(clippy::doc_markdown)]

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

fn get_enemy_state(server: &mut App) -> Option<NetEnemyState> {
    server
        .world_mut()
        .query::<&carcinisation_net::components::NetEnemy>()
        .iter(server.world())
        .next()
        .map(|e| e.state)
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

fn force_player_attack(server: &mut App, pid: u32, attack: NetAttackId) {
    let mut q = server.world_mut().query::<&mut NetPlayer>();
    for mut p in q.iter_mut(server.world_mut()) {
        if p.player_id.0 == pid {
            p.current_attack = attack;
        }
    }
}

fn tick(server: &mut App) {
    std::thread::sleep(std::time::Duration::from_millis(2));
    server.update();
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Enemy at preferred range maps to HoldingRange, not the old Attack.
#[test]
fn enemy_at_range_uses_holding_state() {
    let port = reserve_port();
    // Enemy at (4.5, 1.5), player at (1.5, 1.5). Distance = 3.0 = preferred_range.
    let mut server = build_server_with_enemy(port, 4.5, 1.5);
    server.update();

    spawn_alive_player(&mut server, 1, 1.5, 1.5);

    // Tick enough for AI to evaluate.
    for _ in 0..100 {
        tick(&mut server);
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

    // Set enemy to 1 HP so one shot kills it.
    {
        let mut q = server
            .world_mut()
            .query::<(&carcinisation_net::components::NetEnemy, &mut NetHealth)>();
        for (_, mut h) in q.iter_mut(server.world_mut()) {
            h.current = 1.0;
        }
    }

    // Fire pistol.
    inject_fire(&mut server, 1);
    for _ in 0..50 {
        tick(&mut server);
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

    // Set enemy to 1 HP.
    {
        let mut q = server
            .world_mut()
            .query::<(&carcinisation_net::components::NetEnemy, &mut NetHealth)>();
        for (_, mut h) in q.iter_mut(server.world_mut()) {
            h.current = 1.0;
        }
    }

    // Fire flamethrower — burn system builds intensity progressively,
    // so more ticks are needed than with old instant-damage model.
    inject_fire(&mut server, 1);
    for _ in 0..300 {
        tick(&mut server);
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

    // Set enemy to 1 HP and kill with pistol.
    {
        let mut q = server
            .world_mut()
            .query::<(&carcinisation_net::components::NetEnemy, &mut NetHealth)>();
        for (_, mut h) in q.iter_mut(server.world_mut()) {
            h.current = 1.0;
        }
    }
    inject_fire(&mut server, 1);
    for _ in 0..50 {
        tick(&mut server);
    }

    // Should be Dying now.
    let state = get_enemy_state(&mut server).unwrap();
    assert!(
        matches!(state, NetEnemyState::Dying { .. }),
        "should be Dying after lethal damage, got {state:?}"
    );

    // Wait for death timer (0.5s = ~250 ticks at 2ms).
    for _ in 0..400 {
        tick(&mut server);
    }

    let state = get_enemy_state(&mut server).unwrap();
    assert_eq!(
        state,
        NetEnemyState::Dead { burn: false },
        "should be Dead {{ burn: false }} after pistol death timer"
    );
}
