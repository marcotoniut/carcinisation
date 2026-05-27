//! Flamethrower, mosquiton AI, and default-map combat integration tests.
//!
//! Networked client+server tests that exercise the flamethrower weapon,
//! mosquiton AI movement/targeting, and map-authored enemy configuration.
#![allow(
    clippy::doc_markdown,
    clippy::float_cmp,
    clippy::needless_pass_by_value
)]

mod common;

use std::net::SocketAddr;

use bevy::prelude::*;
use carcinisation_fps_core::{
    map::{EntitySpawnData, EntitySpawnKind, test_map},
    raycast::cast_ray,
};
use carcinisation_net::components::NetEnemy;
use carcinisation_net::{
    NetAttackId, NetEnemyState, NetEnemyType, NetHealth, NetPlayer, PlayerId, PlayerNetState,
};
use carcinisation_server::{
    ServerPlugin,
    systems::{ServerEnemyAiConfig, ServerMosquitonSimConfig},
};
use common::combat::{
    any_enemy_damaged, build_combat_client, build_combat_server, build_default_map_server,
    build_flame_server, client_received_flame_event, enemy_positions_by_object_id,
    get_player_attack, get_player_id, is_flame_active, load_default_map_data, open_test_map,
    queue_fire, queue_idle, queue_switch, wait_for_client_enemy_position, wait_for_player,
};
use common::{build_server_app, get_enemy_health, reserve_port, tick_with_sleep};

// ---------------------------------------------------------------------------
// Default-map / AI tests
// ---------------------------------------------------------------------------

#[test]
fn default_map_spawns_net_enemy_for_each_combat_entity() {
    let map_data = load_default_map_data();
    let expected = map_data
        .entities
        .iter()
        .filter(|entity| entity.kind.is_enemy())
        .count();
    assert_eq!(expected, 9);

    let port = reserve_port();
    let mut server = build_default_map_server(port);
    server.update();

    let spawned = server
        .world_mut()
        .query::<&NetEnemy>()
        .iter(server.world())
        .count();
    assert_eq!(spawned, expected);

    let mosquitons = server
        .world_mut()
        .query::<&NetEnemy>()
        .iter(server.world())
        .filter(|enemy| enemy.enemy_type == NetEnemyType::Mosquiton)
        .count();
    assert_eq!(mosquitons, 6);
}

#[test]
fn default_map_ai_diagnostic_classifies_or_moves_mosquitons() {
    let map_data = load_default_map_data();
    let start = map_data
        .player_starts
        .first()
        .copied()
        .expect("default map should define player start");
    let port = reserve_port();
    let mut server = build_default_map_server(port);
    server.update();
    server.world_mut().spawn(NetPlayer {
        player_id: PlayerId(1),
        position: Vec2::new(start.x, start.y),
        angle: start.angle_deg.to_radians(),
        current_attack: NetAttackId::None,
        state: PlayerNetState::Alive,
        flame_active: false,
        avatar_palette_variant: None,
    });

    let before = enemy_positions_by_object_id(&mut server);
    let mut diagnostics = Vec::new();
    // 30 ticks at 35 ms ≈ 1 s game time at 30 Hz FixedUpdate.
    for _ in 0..30 {
        std::thread::sleep(std::time::Duration::from_millis(35));
        server.update();
    }
    let after = enemy_positions_by_object_id(&mut server);

    let mut moved = 0_usize;
    let mut correctly_held = 0_usize;
    let mut outside_aggro = 0_usize;
    let mut mosquiton_count = 0_usize;
    for ((object_id, before_pos, _before_state), (_, after_pos, _after_state)) in
        before.iter().zip(after.iter())
    {
        let Some(config) = server
            .world_mut()
            .query::<(&NetEnemy, &ServerEnemyAiConfig)>()
            .iter(server.world())
            .find(|(enemy, _)| enemy.object_id.0 == *object_id)
            .map(|(_, config)| config.0)
        else {
            continue;
        };
        mosquiton_count += 1;
        let distance = before_pos.distance(Vec2::new(start.x, start.y));
        let delta = after_pos.distance(*before_pos);
        if delta > 0.01 {
            moved += 1;
        } else if distance <= config.preferred_range + config.preferred_range_hysteresis {
            correctly_held += 1;
        } else if distance > config.aggro_range {
            outside_aggro += 1;
        }
        diagnostics.push(format!(
            "obj={object_id} delta={delta:.3} distance={distance:.3} speed={:.2} preferred={:.2} aggro={:.2}",
            config.move_speed, config.preferred_range, config.aggro_range
        ));
    }

    assert_eq!(
        mosquiton_count, 6,
        "default map should have 6 Mosquitons with AI config"
    );
    assert!(
        moved > 0,
        "at least one default-map Mosquiton should move; held={correctly_held}, outside_aggro={outside_aggro}; diagnostics:\n{}",
        diagnostics.join("\n")
    );
}

#[test]
fn map_authored_mosquiton_speed_is_preserved_on_server_spawn() {
    let entities = vec![EntitySpawnData {
        kind: EntitySpawnKind::Mosquiton {
            health: 100,
            speed: 2.75,
        },
        x: 1.5,
        y: 1.5,
    }];
    let port = reserve_port();
    let mut server = build_server_app(ServerPlugin {
        port,
        map: test_map(),
        entities,
        player_starts: Vec::new(),
        admin_socket: None,
        instance_name: "test".to_string(),
        map_path: "test_map".to_string(),
    });
    server.update();

    let speed = server
        .world_mut()
        .query::<(&NetEnemy, &ServerMosquitonSimConfig)>()
        .iter(server.world())
        .find(|(enemy, _)| enemy.enemy_type == NetEnemyType::Mosquiton)
        .map(|(_, config)| config.0.move_speed)
        .expect("spawned Mosquiton should carry ServerMosquitonSimConfig");

    assert!((speed - 2.75).abs() < 0.001);
}

#[test]
fn open_map_mosquiton_reaches_preferred_range_then_holds() {
    let entities = vec![EntitySpawnData {
        kind: EntitySpawnKind::Mosquiton {
            health: 100,
            speed: 3.0,
        },
        x: 6.5,
        y: 1.5,
    }];
    let port = reserve_port();
    let mut server = build_server_app(ServerPlugin {
        port,
        map: open_test_map(9, 4),
        entities,
        player_starts: Vec::new(),
        admin_socket: None,
        instance_name: "test".to_string(),
        map_path: "test_map".to_string(),
    });
    server.update();
    let player_position = Vec2::new(1.5, 1.5);
    server.world_mut().spawn(NetPlayer {
        player_id: PlayerId(1),
        position: player_position,
        angle: 0.0,
        current_attack: NetAttackId::None,
        state: PlayerNetState::Alive,
        flame_active: false,
        avatar_palette_variant: None,
    });

    let mut previous_distance = server
        .world_mut()
        .query::<&NetEnemy>()
        .iter(server.world())
        .next()
        .expect("enemy should spawn")
        .position
        .distance(player_position);
    let mut saw_chasing = false;
    let mut saw_holding = false;
    let mut diagnostics = Vec::new();

    // 30 ticks at 35 ms ≈ 1 s game time at 30 Hz.
    for _ in 0..30 {
        std::thread::sleep(std::time::Duration::from_millis(35));
        server.update();
        let enemy = server
            .world_mut()
            .query::<&NetEnemy>()
            .iter(server.world())
            .next()
            .cloned()
            .expect("enemy should remain");
        let distance = enemy.position.distance(player_position);
        diagnostics.push(format!(
            "pos={:?} distance={distance:.3} state={:?}",
            enemy.position, enemy.state
        ));
        assert!(
            distance <= previous_distance + 0.1,
            "distance should not increase significantly: prev={previous_distance}, current={distance}; diagnostics:\n{}",
            diagnostics.join("\n")
        );
        saw_chasing |= enemy.state == NetEnemyState::Chase;
        saw_holding |= enemy.state == NetEnemyState::HoldingRange;
        previous_distance = distance;
    }

    assert!(
        saw_chasing,
        "open-map Mosquiton should chase before reaching hold range; diagnostics:\n{}",
        diagnostics.join("\n")
    );
    let preferred = carcinisation_fps_core::FpsCombatConfig::default().mosquiton_preferred_range;
    assert!(
        (previous_distance - preferred).abs() < 1.0,
        "should reach near preferred range ({preferred}); final_distance={previous_distance}; diagnostics:\n{}",
        diagnostics.join("\n")
    );
    let final_state = server
        .world_mut()
        .query::<&NetEnemy>()
        .iter(server.world())
        .next()
        .expect("enemy should remain")
        .state;
    assert_eq!(
        final_state,
        NetEnemyState::HoldingRange,
        "Mosquiton should enter hold/attack at preferred range; diagnostics:\n{}",
        diagnostics.join("\n")
    );
    assert!(
        saw_holding,
        "open-map Mosquiton should hold/attack near preferred range; diagnostics:\n{}",
        diagnostics.join("\n")
    );
}

/// Mosquiton chases toward player when far from preferred range.
#[test]
fn server_ai_updates_live_mosquiton_position() {
    let entities = vec![EntitySpawnData {
        kind: EntitySpawnKind::Mosquiton {
            health: 100,
            speed: 1.2,
        },
        x: 1.5,
        y: 1.5,
    }];
    let port = reserve_port();
    let mut server = build_server_app(ServerPlugin {
        port,
        map: open_test_map(10, 4),
        entities,
        player_starts: Vec::new(),
        admin_socket: None,
        instance_name: "test".to_string(),
        map_path: "test_map".to_string(),
    });
    server.update();
    server.world_mut().spawn(NetPlayer {
        player_id: PlayerId(1),
        position: Vec2::new(6.5, 1.5),
        angle: 0.0,
        current_attack: NetAttackId::None,
        state: PlayerNetState::Alive,
        flame_active: false,
        avatar_palette_variant: None,
    });

    let before = server
        .world_mut()
        .query::<&NetEnemy>()
        .iter(server.world())
        .next()
        .expect("enemy should spawn")
        .position;
    for _ in 0..20 {
        std::thread::sleep(std::time::Duration::from_millis(3));
        server.update();
    }
    let enemy = server
        .world_mut()
        .query::<&NetEnemy>()
        .iter(server.world())
        .next()
        .cloned()
        .expect("enemy should remain");

    assert!(
        enemy.position.x > before.x,
        "mosquiton should move east toward player: {:?} -> {:?}",
        before,
        enemy.position
    );
    assert_eq!(enemy.state, NetEnemyState::Chase);
}

#[test]
fn map_authored_speed_changes_mosquiton_movement_distance() {
    let entities = vec![
        EntitySpawnData {
            kind: EntitySpawnKind::Mosquiton {
                health: 100,
                speed: 0.5,
            },
            x: 1.5,
            y: 2.0,
        },
        EntitySpawnData {
            kind: EntitySpawnKind::Mosquiton {
                health: 100,
                speed: 2.0,
            },
            x: 1.5,
            y: 3.0,
        },
    ];
    let port = reserve_port();
    let mut server = build_server_app(ServerPlugin {
        port,
        map: open_test_map(10, 6),
        entities,
        player_starts: Vec::new(),
        admin_socket: None,
        instance_name: "test".to_string(),
        map_path: "test_map".to_string(),
    });
    server.update();
    server.world_mut().spawn(NetPlayer {
        player_id: PlayerId(1),
        position: Vec2::new(8.5, 2.5),
        angle: 0.0,
        current_attack: NetAttackId::None,
        state: PlayerNetState::Alive,
        flame_active: false,
        avatar_palette_variant: None,
    });

    let before: Vec<(f32, Vec2)> = server
        .world_mut()
        .query::<(&NetEnemy, &ServerEnemyAiConfig)>()
        .iter(server.world())
        .map(|(enemy, config)| (config.0.move_speed, enemy.position))
        .collect();

    // 10 ticks at 35 ms ≈ 0.35 s game time at 30 Hz.
    for _ in 0..10 {
        std::thread::sleep(std::time::Duration::from_millis(35));
        server.update();
    }

    let after: Vec<(f32, Vec2)> = server
        .world_mut()
        .query::<(&NetEnemy, &ServerEnemyAiConfig)>()
        .iter(server.world())
        .map(|(enemy, config)| (config.0.move_speed, enemy.position))
        .collect();

    let moved_by_speed = |speed: f32| {
        let start = before
            .iter()
            .find(|(s, _)| (*s - speed).abs() < 0.001)
            .expect("speed should exist")
            .1;
        let end = after
            .iter()
            .find(|(s, _)| (*s - speed).abs() < 0.001)
            .expect("speed should exist")
            .1;
        end.distance(start)
    };

    let slow_distance = moved_by_speed(0.5);
    let fast_distance = moved_by_speed(2.0);
    assert!(
        fast_distance > slow_distance * 2.0,
        "fast Mosquiton should move farther: slow={slow_distance}, fast={fast_distance}"
    );
}

#[test]
fn client_receives_replicated_mosquiton_position_updates() {
    let entities = vec![EntitySpawnData {
        kind: EntitySpawnKind::Mosquiton {
            health: 100,
            speed: 2.0,
        },
        x: 7.5,
        y: 2.5,
    }];
    let port = reserve_port();
    let mut server = build_server_app(ServerPlugin {
        port,
        map: open_test_map(10, 5),
        entities,
        player_starts: Vec::new(),
        admin_socket: None,
        instance_name: "test".to_string(),
        map_path: "test_map".to_string(),
    });
    server.update();

    let addr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), port);
    let mut client = build_combat_client(addr);
    client.update();
    assert!(wait_for_player(&mut server, &mut client));

    let initial = wait_for_client_enemy_position(&mut server, &mut client)
        .expect("client should receive initial NetEnemy");

    for _ in 0..80 {
        tick_with_sleep(&mut server, &mut client);
    }

    let updated = wait_for_client_enemy_position(&mut server, &mut client)
        .expect("client should still have replicated NetEnemy");
    assert!(
        updated.distance(initial) > 0.05,
        "client should receive changed NetEnemy.position: {initial:?} -> {updated:?}"
    );
}

#[test]
fn server_ai_does_not_move_dead_mosquiton() {
    let entities = vec![EntitySpawnData {
        kind: EntitySpawnKind::Mosquiton {
            health: 100,
            speed: 1.2,
        },
        x: 1.5,
        y: 1.5,
    }];
    let port = reserve_port();
    let mut server = build_server_app(ServerPlugin {
        port,
        map: test_map(),
        entities,
        player_starts: Vec::new(),
        admin_socket: None,
        instance_name: "test".to_string(),
        map_path: "test_map".to_string(),
    });
    server.update();
    server.world_mut().spawn(NetPlayer {
        player_id: PlayerId(1),
        position: Vec2::new(5.5, 1.5),
        angle: 0.0,
        current_attack: NetAttackId::None,
        state: PlayerNetState::Alive,
        flame_active: false,
        avatar_palette_variant: None,
    });

    let before = {
        let mut query = server
            .world_mut()
            .query::<(&mut NetEnemy, &mut NetHealth)>();
        let (mut enemy, mut health) = query
            .iter_mut(server.world_mut())
            .next()
            .expect("enemy should spawn");
        enemy.state = NetEnemyState::Dead { burn: false };
        health.current = 0.0;
        enemy.position
    };
    for _ in 0..20 {
        std::thread::sleep(std::time::Duration::from_millis(3));
        server.update();
    }
    let enemy = server
        .world_mut()
        .query::<&NetEnemy>()
        .iter(server.world())
        .next()
        .cloned()
        .expect("enemy should remain");

    assert_eq!(enemy.position, before);
    assert!(matches!(enemy.state, NetEnemyState::Dead { .. }));
}

// ---------------------------------------------------------------------------
// Flamethrower tests
// ---------------------------------------------------------------------------

#[test]
fn default_map_first_spawn_has_live_flamethrower_target() {
    let flame_cfg = carcinisation_fps_core::PlayerFlamethrowerConfig::load();

    let map_data = load_default_map_data();
    let expected_start = map_data
        .player_starts
        .first()
        .copied()
        .expect("default multiplayer map must define player_start");

    let port = reserve_port();
    let mut server = build_server_app(ServerPlugin {
        port,
        map: map_data.map.clone(),
        entities: map_data.entities.clone(),
        player_starts: map_data.player_starts.clone(),
        admin_socket: None,
        instance_name: "test".to_string(),
        map_path: "test_map".to_string(),
    });
    server.update();

    let addr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), port);
    let mut client = build_combat_client(addr);
    client.update();

    assert!(wait_for_player(&mut server, &mut client));
    let player = server
        .world_mut()
        .query::<&NetPlayer>()
        .iter(server.world())
        .next()
        .cloned()
        .expect("server should spawn first NetPlayer");

    assert!(
        (player.position.x - expected_start.x).abs() < 0.001
            && (player.position.y - expected_start.y).abs() < 0.001,
        "first server spawn must match map player_start: server={:?}, map=({}, {})",
        player.position,
        expected_start.x,
        expected_start.y
    );
    assert!(
        (player.angle - expected_start.angle_deg.to_radians()).abs() < 0.001,
        "first server spawn angle must match map player_start: server={} rad, map={} deg",
        player.angle,
        expected_start.angle_deg
    );

    let dir = Vec2::new(player.angle.cos(), player.angle.sin());
    let hit_half_w_sq = flame_cfg.hit_half_width * flame_cfg.hit_half_width;
    let mut diagnostics = Vec::new();
    let has_target = map_data
        .entities
        .iter()
        .filter(|spawn| spawn.kind.is_enemy())
        .any(|enemy| {
            let enemy_pos = Vec2::new(enemy.x, enemy.y);
            let to_enemy = enemy_pos - player.position;
            let along = to_enemy.dot(dir);
            let perp_sq = (to_enemy - dir * along).length_squared();
            let to_dir = to_enemy.normalize();
            let ray_hit = cast_ray(&map_data.map, player.position, to_dir);
            let in_range = (0.01..=flame_cfg.range).contains(&along);
            let in_line = perp_sq <= hit_half_w_sq;
            let has_los = ray_hit.distance >= to_enemy.length();
            diagnostics.push(format!(
                "enemy=({:.1},{:.1}) along={:.3} perp={:.3} in_range={} in_line={} los={:.3} has_los={}",
                enemy.x, enemy.y, along, perp_sq.sqrt(), in_range, in_line, ray_hit.distance, has_los
            ));
            in_range && in_line && has_los
        });

    assert!(
        has_target,
        "default map first spawn should have a valid opening flamethrower target; diagnostics:\n{}",
        diagnostics.join("\n")
    );
}

#[test]
fn default_map_flamethrower_damages_from_spawn() {
    let port = reserve_port();
    let mut server = build_default_map_server(port);
    server.update();

    let addr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), port);
    let mut client = build_combat_client(addr);
    client.update();

    assert!(wait_for_player(&mut server, &mut client));
    assert!(!any_enemy_damaged(&mut server));

    queue_switch(&mut client, 1);
    for _ in 0..20 {
        tick_with_sleep(&mut server, &mut client);
    }
    assert_eq!(
        get_player_attack(&mut server),
        Some(NetAttackId::Projectile)
    );

    // Burn system builds intensity progressively — more ticks needed than instant DPS.
    for seq in 2..=60 {
        queue_fire(&mut client, seq);
        tick_with_sleep(&mut server, &mut client);
    }
    for _ in 0..40 {
        tick_with_sleep(&mut server, &mut client);
    }

    assert!(
        any_enemy_damaged(&mut server),
        "holding flamethrower from the default map spawn should damage at least one NetEnemy"
    );
}

#[test]
fn flamethrower_damages_over_ticks() {
    let port = reserve_port();
    let mut server = build_flame_server(port);
    server.update();

    let addr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), port);
    let mut client = build_combat_client(addr);
    client.update();

    assert!(wait_for_player(&mut server, &mut client));

    queue_switch(&mut client, 1);
    for _ in 0..10 {
        tick_with_sleep(&mut server, &mut client);
    }

    let initial_hp = get_enemy_health(&mut server).unwrap();

    // Burn system builds intensity progressively — hold fire for longer.
    for seq in 2..=60 {
        queue_fire(&mut client, seq);
        tick_with_sleep(&mut server, &mut client);
    }
    for _ in 0..40 {
        tick_with_sleep(&mut server, &mut client);
    }

    let hp_after = get_enemy_health(&mut server).unwrap();
    let damage = initial_hp - hp_after;

    assert!(
        damage > 0.0,
        "flamethrower should deal continuous damage: {initial_hp} → {hp_after} (dmg={damage})"
    );
}

#[test]
fn flamethrower_e2e_switch_fire_release_uses_server_state() {
    let port = reserve_port();
    let mut server = build_flame_server(port);
    server.update();

    let addr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), port);
    let mut client = build_combat_client(addr);
    client.update();

    assert!(wait_for_player(&mut server, &mut client));
    let player_id = get_player_id(&mut server).unwrap();
    assert_eq!(get_player_attack(&mut server), Some(NetAttackId::None));

    queue_switch(&mut client, 1);
    for _ in 0..20 {
        tick_with_sleep(&mut server, &mut client);
    }
    assert_eq!(
        get_player_attack(&mut server),
        Some(NetAttackId::Projectile),
        "BTN_SWITCH should change server NetPlayer.current_attack to flamethrower"
    );

    let initial_hp = get_enemy_health(&mut server).unwrap();
    assert!(!is_flame_active(&server, player_id));

    // Burn system builds intensity progressively. Fire for up to 200 ticks,
    // with early exit once flame_active is set on the server.
    let mut flame_activated = false;
    for seq in 2..=200 {
        queue_fire(&mut client, seq);
        tick_with_sleep(&mut server, &mut client);
        if is_flame_active(&server, player_id) {
            flame_activated = true;
            // Continue a few more ticks to accumulate damage.
            for _ in 0..20 {
                tick_with_sleep(&mut server, &mut client);
            }
            break;
        }
    }

    assert!(
        flame_activated,
        "flamethrower branch should mark the player as actively flaming"
    );
    assert!(
        client_received_flame_event(&client, player_id, true),
        "client should receive FlameActive(true) for the local PlayerId"
    );
    let hp_after_fire = get_enemy_health(&mut server).unwrap();
    assert!(
        hp_after_fire < initial_hp,
        "flamethrower branch should mutate NetEnemy/NetHealth: {initial_hp} -> {hp_after_fire}"
    );

    queue_idle(&mut client, 13);
    for _ in 0..20 {
        tick_with_sleep(&mut server, &mut client);
    }
    assert!(
        !is_flame_active(&server, player_id),
        "release should stop FlameActive on the server"
    );
    assert!(
        client_received_flame_event(&client, player_id, false),
        "client should receive FlameActive(false) for the local PlayerId"
    );

    // Burn intensity decays + ground fire expires after ~15 s.
    // 1200 ticks at 2 ms ≈ 72 FixedUpdate cycles at 30 Hz ≈ 2.4 s game time.
    for _ in 0..1200 {
        tick_with_sleep(&mut server, &mut client);
    }
    let hp_stable = get_enemy_health(&mut server).unwrap();
    for _ in 0..200 {
        tick_with_sleep(&mut server, &mut client);
    }
    let hp_final = get_enemy_health(&mut server).unwrap();
    // Allow 1 HP tolerance for residual damage accumulator flush.
    assert!(
        (hp_stable - hp_final).abs() <= 1.0,
        "NetHealth should stabilise after burn decays: {hp_stable} -> {hp_final}"
    );
}

#[test]
fn net_player_flame_active_tracks_server_state() {
    let port = reserve_port();
    let mut server = build_flame_server(port);
    server.update();

    let addr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), port);
    let mut client = build_combat_client(addr);
    client.update();

    assert!(wait_for_player(&mut server, &mut client));

    queue_switch(&mut client, 1);
    for _ in 0..20 {
        tick_with_sleep(&mut server, &mut client);
    }

    let flame_before = server
        .world_mut()
        .query::<&NetPlayer>()
        .iter(server.world())
        .next()
        .map(|p| p.flame_active);
    assert_eq!(
        flame_before,
        Some(false),
        "flame_active should be false before firing"
    );

    for seq in 2..=12 {
        queue_fire(&mut client, seq);
        tick_with_sleep(&mut server, &mut client);
    }
    for _ in 0..20 {
        tick_with_sleep(&mut server, &mut client);
    }

    let flame_during = server
        .world_mut()
        .query::<&NetPlayer>()
        .iter(server.world())
        .next()
        .map(|p| p.flame_active);
    assert_eq!(
        flame_during,
        Some(true),
        "NetPlayer.flame_active should be true while flamethrower is held"
    );

    queue_idle(&mut client, 13);
    for _ in 0..20 {
        tick_with_sleep(&mut server, &mut client);
    }

    let flame_after = server
        .world_mut()
        .query::<&NetPlayer>()
        .iter(server.world())
        .next()
        .map(|p| p.flame_active);
    assert_eq!(
        flame_after,
        Some(false),
        "NetPlayer.flame_active should be false after releasing fire"
    );
}

#[test]
fn flamethrower_stops_on_release() {
    let port = reserve_port();
    let mut server = build_flame_server(port);
    server.update();

    let addr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), port);
    let mut client = build_combat_client(addr);
    client.update();

    assert!(wait_for_player(&mut server, &mut client));

    queue_switch(&mut client, 1);
    for _ in 0..10 {
        tick_with_sleep(&mut server, &mut client);
    }

    // Fire briefly.
    for seq in 2..=5 {
        queue_fire(&mut client, seq);
        tick_with_sleep(&mut server, &mut client);
    }
    for _ in 0..10 {
        tick_with_sleep(&mut server, &mut client);
    }

    queue_idle(&mut client, 6);
    for _ in 0..10 {
        tick_with_sleep(&mut server, &mut client);
    }

    let hp_at_release = get_enemy_health(&mut server).unwrap();

    for _ in 0..20 {
        tick_with_sleep(&mut server, &mut client);
    }

    let hp_final = get_enemy_health(&mut server).unwrap();
    assert_eq!(
        hp_at_release, hp_final,
        "damage should stop after fire release: {hp_at_release} → {hp_final}"
    );
}

#[test]
fn pistol_still_uses_cooldown_after_switch() {
    let port = reserve_port();
    let mut server = build_combat_server(port);
    server.update();

    let addr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), port);
    let mut client = build_combat_client(addr);
    client.update();

    assert!(wait_for_player(&mut server, &mut client));

    let initial_hp = get_enemy_health(&mut server).unwrap();

    // Switch to flamethrower, then back to pistol.
    queue_switch(&mut client, 1);
    for _ in 0..20 {
        tick_with_sleep(&mut server, &mut client);
    }
    queue_idle(&mut client, 2);
    for _ in 0..5 {
        tick_with_sleep(&mut server, &mut client);
    }
    queue_switch(&mut client, 3);
    for _ in 0..20 {
        tick_with_sleep(&mut server, &mut client);
    }

    // Fire pistol — should use hitscan.
    queue_fire(&mut client, 4);
    for _ in 0..20 {
        tick_with_sleep(&mut server, &mut client);
    }

    let hp_after = get_enemy_health(&mut server).unwrap();
    let damage = initial_hp - hp_after;

    let expected = carcinisation_fps_core::FpsCombatConfig::default().hitscan_damage;
    assert!(
        (damage - expected).abs() < 1.0,
        "pistol should do hitscan damage ({expected}): got {damage}"
    );
}
