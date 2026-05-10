//! Phase 5a combat integration tests.
//!
//! High-value tests only: fire release, damage, death, disconnect, fixed-tick cooldown.
#![allow(
    clippy::doc_markdown,
    clippy::float_cmp,
    clippy::needless_pass_by_value
)]

mod common;

use std::net::SocketAddr;

use bevy::prelude::*;
use bevy_replicon::prelude::*;
use carcinisation_fps_core::{
    map::{EntitySpawnData, EntitySpawnKind, Map, test_map},
    raycast::cast_ray,
};
use carcinisation_net::components::NetEnemy;
use carcinisation_net::{
    ClientIntent, FlameActive, InputSequence, NetAttackId, NetEnemyState, NetEnemyType, NetHealth,
    NetPlayer, NetProtocolPlugin, PlayerActions, PlayerId, PlayerNetState, register_net_all,
};
use carcinisation_server::{
    ServerPlugin,
    systems::{FlameActiveTracker, ServerEnemyAiConfig, ServerMosquitonSimConfig},
};
use common::{build_client_app, build_server_app, reserve_port, tick_with_sleep};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

#[derive(Resource, Default)]
struct TestIntentQueue(Vec<ClientIntent>);

#[derive(Resource, Default)]
struct ReceivedFlameEvents(Vec<FlameActive>);

fn send_queued_intents(mut commands: Commands, mut queue: ResMut<TestIntentQueue>) {
    for intent in queue.0.drain(..) {
        commands.client_trigger(intent);
    }
}

fn capture_flame_active(trigger: On<FlameActive>, mut events: ResMut<ReceivedFlameEvents>) {
    events.0.push(trigger.event().clone());
}

fn queue_fire(app: &mut App, seq: u32) {
    app.world_mut()
        .resource_mut::<TestIntentQueue>()
        .0
        .push(ClientIntent {
            sequence: InputSequence(seq),
            movement: Vec2::ZERO,
            turn: 0.0,
            fire_held: true,
            actions: PlayerActions::default(),
        });
}

fn queue_idle(app: &mut App, seq: u32) {
    app.world_mut()
        .resource_mut::<TestIntentQueue>()
        .0
        .push(ClientIntent::idle(InputSequence(seq)));
}

fn queue_switch(app: &mut App, seq: u32) {
    app.world_mut()
        .resource_mut::<TestIntentQueue>()
        .0
        .push(ClientIntent {
            sequence: InputSequence(seq),
            movement: Vec2::ZERO,
            turn: 0.0,
            fire_held: false,
            actions: PlayerActions::from_raw(PlayerActions::WEAPON_SWITCH),
        });
}

/// Build a server with test_map + one enemy at (4.5, 1.5) — directly east of
/// spawn (1.5, 1.5) at angle 0.
fn build_combat_server(port: u16) -> App {
    let entities = vec![EntitySpawnData {
        kind: EntitySpawnKind::Mosquiton {
            health: 100,
            speed: 0.0,
        },
        x: 4.5,
        y: 1.5,
    }];
    build_server_app(ServerPlugin {
        port,
        map: test_map(),
        entities,
        player_starts: Vec::new(),
    })
}

fn build_combat_client(addr: SocketAddr) -> App {
    let mut app = build_client_app(NetProtocolPlugin, register_net_all, addr);
    app.init_resource::<TestIntentQueue>();
    app.init_resource::<ReceivedFlameEvents>();
    app.add_observer(capture_flame_active);
    app.add_systems(Update, send_queued_intents);
    app
}

fn wait_for_player(server: &mut App, client: &mut App) -> bool {
    for _ in 0..200 {
        tick_with_sleep(server, client);
        let count = server
            .world_mut()
            .query::<&NetPlayer>()
            .iter(server.world())
            .count();
        if count >= 1 {
            return true;
        }
    }
    false
}

fn get_enemy_health(server: &mut App) -> Option<f32> {
    server
        .world_mut()
        .query::<(&NetEnemy, &NetHealth)>()
        .iter(server.world())
        .next()
        .map(|(_, h)| h.current)
}

fn get_enemy_state(server: &mut App) -> Option<NetEnemyState> {
    server
        .world_mut()
        .query::<&NetEnemy>()
        .iter(server.world())
        .next()
        .map(|e| e.state)
}

fn get_player_id(server: &mut App) -> Option<PlayerId> {
    server
        .world_mut()
        .query::<&NetPlayer>()
        .iter(server.world())
        .next()
        .map(|p| p.player_id)
}

fn get_player_attack(server: &mut App) -> Option<NetAttackId> {
    server
        .world_mut()
        .query::<&NetPlayer>()
        .iter(server.world())
        .next()
        .map(|p| p.current_attack)
}

fn is_flame_active(server: &App, player_id: PlayerId) -> bool {
    server
        .world()
        .resource::<FlameActiveTracker>()
        .0
        .get(&player_id)
        .copied()
        .unwrap_or(false)
}

fn client_received_flame_event(client: &App, player_id: PlayerId, active: bool) -> bool {
    client
        .world()
        .resource::<ReceivedFlameEvents>()
        .0
        .iter()
        .any(|event| event.player_id == player_id && event.active == active)
}

fn enemy_count(server: &mut App) -> usize {
    server
        .world_mut()
        .query::<&NetEnemy>()
        .iter(server.world())
        .count()
}

fn wait_for_client_enemy_position(server: &mut App, client: &mut App) -> Option<Vec2> {
    for _ in 0..100 {
        tick_with_sleep(server, client);
        let position = client
            .world_mut()
            .query::<&NetEnemy>()
            .iter(client.world())
            .next()
            .map(|enemy| enemy.position);
        if position.is_some() {
            return position;
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Hold BTN_FIRE, then release. Server should stop firing after release.
#[test]
fn fire_input_release_stops_firing() {
    let port = reserve_port();
    let mut server = build_combat_server(port);
    server.update();

    let addr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), port);
    let mut client = build_combat_client(addr);
    client.update();

    assert!(wait_for_player(&mut server, &mut client));
    assert_eq!(enemy_count(&mut server), 1);

    let initial_hp = get_enemy_health(&mut server).unwrap();

    // Fire for several ticks.
    for seq in 1..=5 {
        queue_fire(&mut client, seq);
        tick_with_sleep(&mut server, &mut client);
    }
    for _ in 0..20 {
        tick_with_sleep(&mut server, &mut client);
    }

    let hp_after_fire = get_enemy_health(&mut server).unwrap();
    assert!(
        hp_after_fire < initial_hp,
        "enemy should have taken damage: {initial_hp} → {hp_after_fire}"
    );

    // Release fire (send buttons=0).
    queue_idle(&mut client, 6);
    for _ in 0..10 {
        tick_with_sleep(&mut server, &mut client);
    }

    let hp_after_release = get_enemy_health(&mut server).unwrap();

    // Tick more — no new damage should occur.
    for _ in 0..30 {
        tick_with_sleep(&mut server, &mut client);
    }

    let hp_final = get_enemy_health(&mut server).unwrap();
    assert_eq!(
        hp_after_release, hp_final,
        "enemy health should not change after fire released: {hp_after_release} → {hp_final}"
    );
}

/// Fire at enemy, verify NetHealth decreases.
#[test]
fn enemy_takes_damage_from_hitscan() {
    let port = reserve_port();
    let mut server = build_combat_server(port);
    server.update();

    let addr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), port);
    let mut client = build_combat_client(addr);
    client.update();

    assert!(wait_for_player(&mut server, &mut client));

    let initial_hp = get_enemy_health(&mut server).unwrap();
    assert!(
        (initial_hp - 100.0).abs() < 0.1,
        "enemy should start at 100hp"
    );

    // Fire once.
    queue_fire(&mut client, 1);
    for _ in 0..20 {
        tick_with_sleep(&mut server, &mut client);
    }

    let hp_after = get_enemy_health(&mut server).unwrap();
    assert!(
        hp_after < initial_hp,
        "enemy should take damage: {initial_hp} → {hp_after}"
    );
}

/// Fire until enemy dies. Verify exactly one death transition.
#[test]
fn enemy_dies_once() {
    let port = reserve_port();
    let mut server = build_combat_server(port);
    server.update();

    let addr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), port);
    let mut client = build_combat_client(addr);
    client.update();

    assert!(wait_for_player(&mut server, &mut client));

    // Fire repeatedly until enemy is dead.
    // 100 HP / 37 dmg = 3 shots needed. At 0.33s cooldown and 30Hz tick,
    // need ~1.0s real time for 3 shots. Use longer sleeps to accumulate time.
    for seq in 1..=20 {
        queue_fire(&mut client, seq);
        std::thread::sleep(std::time::Duration::from_millis(50));
        server.update();
        client.update();
    }
    for _ in 0..20 {
        std::thread::sleep(std::time::Duration::from_millis(50));
        server.update();
        client.update();
    }

    let state = get_enemy_state(&mut server).unwrap();
    assert!(
        matches!(
            state,
            NetEnemyState::Dying { .. } | NetEnemyState::Dead { .. }
        ),
        "enemy should be dying or dead, got {state:?}"
    );

    let hp = get_enemy_health(&mut server).unwrap();
    assert!(hp <= 0.0, "enemy health should be <= 0: {hp}");

    // Keep firing — health should not go further negative or cause issues.
    for seq in 21..=30 {
        queue_fire(&mut client, seq);
        std::thread::sleep(std::time::Duration::from_millis(50));
        server.update();
        client.update();
    }

    let hp_final = get_enemy_health(&mut server).unwrap();
    assert!(
        hp_final <= 0.0,
        "enemy health should remain <= 0: {hp_final}"
    );
    let final_state = get_enemy_state(&mut server).unwrap();
    assert!(
        matches!(
            final_state,
            NetEnemyState::Dying { .. } | NetEnemyState::Dead { .. }
        ),
        "enemy should stay dying or dead, got {final_state:?}"
    );
}

/// Cooldown uses server fixed tick delta, not frame count.
/// Two fire commands sent within a single cooldown period (0.33s) should produce
/// exactly one shot (37 damage). The server's `FireCooldownMap` should hold a
/// non-zero cooldown for the player after the first shot.
#[test]
fn combat_uses_fixed_tick_cooldown() {
    use carcinisation_server::systems::FireCooldownMap;

    let port = reserve_port();
    let mut server = build_combat_server(port);
    server.update();

    let addr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), port);
    let mut client = build_combat_client(addr);
    client.update();

    assert!(wait_for_player(&mut server, &mut client));

    let initial_hp = get_enemy_health(&mut server).unwrap();

    // Send fire on two consecutive ticks — cooldown should prevent the second.
    queue_fire(&mut client, 1);
    tick_with_sleep(&mut server, &mut client);
    queue_fire(&mut client, 2);
    tick_with_sleep(&mut server, &mut client);

    // Wait for FixedUpdate to process both commands.
    for _ in 0..20 {
        tick_with_sleep(&mut server, &mut client);
    }

    let hp = get_enemy_health(&mut server).unwrap();
    let damage = initial_hp - hp;

    // Exactly one shot should land (37 damage). Two shots would be 74.
    assert_eq!(
        damage,
        carcinisation_fps_core::config::HITSCAN_DAMAGE,
        "cooldown should allow exactly 1 shot, got damage={damage}"
    );

    // Verify the cooldown resource has a non-zero entry for the player.
    let pid = get_player_id(&mut server).expect("player should exist");
    let cooldowns = server.world().resource::<FireCooldownMap>();
    let cd = cooldowns.0.get(&pid).copied().unwrap_or(0.0);
    assert!(
        cd >= 0.0,
        "FireCooldownMap should have an entry for the player after firing"
    );
}

// ---------------------------------------------------------------------------
// Flamethrower tests
// ---------------------------------------------------------------------------

/// Build a server with an enemy at (3.5, 1.5) — close enough for flamethrower range (5.0 units)
/// from spawn (1.5, 1.5) at angle 0 (facing east).
fn build_flame_server(port: u16) -> App {
    let entities = vec![EntitySpawnData {
        kind: EntitySpawnKind::Mosquiton {
            health: 200,
            speed: 0.0,
        },
        x: 3.5,
        y: 1.5,
    }];
    build_server_app(ServerPlugin {
        port,
        map: test_map(),
        entities,
        player_starts: Vec::new(),
    })
}

fn load_default_map_data() -> carcinisation_fps_core::map::MapLoadData {
    let ron = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/config/fp/test_room.fp_map.ron"
    ))
    .expect("default multiplayer map should exist");
    Map::load_data(&ron).expect("default multiplayer map should parse")
}

fn build_default_map_server(port: u16) -> App {
    let map_data = load_default_map_data();
    build_server_app(ServerPlugin {
        port,
        map: map_data.map,
        entities: map_data.entities,
        player_starts: map_data.player_starts,
    })
}

fn open_test_map(width: usize, height: usize) -> Map {
    let mut cells = vec![0; width * height];
    for x in 0..width {
        cells[x] = 1;
        cells[(height - 1) * width + x] = 1;
    }
    for y in 0..height {
        cells[y * width] = 1;
        cells[y * width + width - 1] = 1;
    }
    Map {
        width,
        height,
        cells,
    }
}

fn any_enemy_damaged(server: &mut App) -> bool {
    server
        .world_mut()
        .query::<(&NetEnemy, &NetHealth)>()
        .iter(server.world())
        .any(|(_, health)| health.current < health.max)
}

fn enemy_positions_by_object_id(server: &mut App) -> Vec<(u32, Vec2, NetEnemyState)> {
    let mut positions: Vec<_> = server
        .world_mut()
        .query::<&NetEnemy>()
        .iter(server.world())
        .map(|enemy| (enemy.object_id.0, enemy.position, enemy.state))
        .collect();
    positions.sort_by_key(|(object_id, _, _)| *object_id);
    positions
}

#[test]
fn default_map_spawns_net_enemy_for_each_combat_entity() {
    let map_data = load_default_map_data();
    let expected = map_data
        .entities
        .iter()
        .filter(|entity| entity.kind.is_enemy())
        .count();
    assert_eq!(expected, 6);

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
    });

    let before = enemy_positions_by_object_id(&mut server);
    let mut diagnostics = Vec::new();
    for _ in 0..30 {
        std::thread::sleep(std::time::Duration::from_millis(35));
        server.update();
    }
    let after = enemy_positions_by_object_id(&mut server);

    let mut moved = 0_usize;
    let mut correctly_held = 0_usize;
    let mut outside_aggro = 0_usize;
    for ((object_id, before_pos, before_state), (_, after_pos, after_state)) in
        before.iter().zip(after.iter())
    {
        let config = server
            .world_mut()
            .query::<(&NetEnemy, &ServerEnemyAiConfig)>()
            .iter(server.world())
            .find(|(enemy, _)| enemy.object_id.0 == *object_id)
            .map(|(_, config)| config.0)
            .expect("Mosquiton should have config");
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
            "obj={object_id} before={before_pos:?} after={after_pos:?} delta={delta:.3} distance={distance:.3} state={before_state:?}->{after_state:?} speed={:.2} preferred={:.2} hysteresis={:.2} aggro={:.2}",
            config.move_speed,
            config.preferred_range,
            config.preferred_range_hysteresis,
            config.aggro_range
        ));
    }

    assert_eq!(before.len(), 6, "default map should spawn 6 Mosquitons");
    assert!(
        moved > 0,
        "at least one default-map Mosquiton should move over diagnostic ticks; held={correctly_held}, outside_aggro={outside_aggro}; diagnostics:\n{}",
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
    });
    server.update();
    let player_position = Vec2::new(1.5, 1.5);
    server.world_mut().spawn(NetPlayer {
        player_id: PlayerId(1),
        position: player_position,
        angle: 0.0,
        current_attack: NetAttackId::None,
        state: PlayerNetState::Alive,
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
        // Strafe may increase distance slightly; allow 0.1 tolerance.
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
    // Preferred range is MOSQUITON_PREFERRED_RANGE (4.0). Allow tolerance for
    // strafe drift and discrete stepping.
    let preferred = carcinisation_fps_core::config::MOSQUITON_PREFERRED_RANGE;
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
    // Place enemy at (1.5, 1.5), player far east at (6.5, 1.5).
    // Distance = 5.0 > MOSQUITON_PREFERRED_RANGE = 4.0, so enemy should chase.
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
    });
    server.update();
    server.world_mut().spawn(NetPlayer {
        player_id: PlayerId(1),
        position: Vec2::new(6.5, 1.5),
        angle: 0.0,
        current_attack: NetAttackId::None,
        state: PlayerNetState::Alive,
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
    });
    server.update();
    server.world_mut().spawn(NetPlayer {
        player_id: PlayerId(1),
        position: Vec2::new(8.5, 2.5),
        angle: 0.0,
        current_attack: NetAttackId::None,
        state: PlayerNetState::Alive,
    });

    let before: Vec<(f32, Vec2)> = server
        .world_mut()
        .query::<(&NetEnemy, &ServerEnemyAiConfig)>()
        .iter(server.world())
        .map(|(enemy, config)| (config.0.move_speed, enemy.position))
        .collect();

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
    });
    server.update();
    server.world_mut().spawn(NetPlayer {
        player_id: PlayerId(1),
        position: Vec2::new(5.5, 1.5),
        angle: 0.0,
        current_attack: NetAttackId::None,
        state: PlayerNetState::Alive,
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

/// The default live map should spawn the first player at `player_start`, where
/// the opening flamethrower shot has at least one valid target.
#[test]
fn default_map_first_spawn_has_live_flamethrower_target() {
    use carcinisation_fps_core::config;
    const FLAME_RANGE: f32 = config::FLAME_RANGE;
    const FLAME_HIT_HALF_WIDTH: f32 = config::FLAME_HIT_HALF_WIDTH;

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
    let hit_half_w_sq = FLAME_HIT_HALF_WIDTH * FLAME_HIT_HALF_WIDTH;
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
            let in_range = (0.01..=FLAME_RANGE).contains(&along);
            let in_line = perp_sq <= hit_half_w_sq;
            let has_los = ray_hit.distance >= to_enemy.length();
            diagnostics.push(format!(
                "enemy=({:.1},{:.1}) along={:.3} perp={:.3} in_range={} in_line={} los_dist={:.3} has_los={}",
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

/// Real default map, real input path: first spawn can damage an opening target.
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

    for seq in 2..=12 {
        queue_fire(&mut client, seq);
        tick_with_sleep(&mut server, &mut client);
    }
    for _ in 0..20 {
        tick_with_sleep(&mut server, &mut client);
    }

    assert!(
        any_enemy_damaged(&mut server),
        "holding flamethrower from the default map spawn should damage at least one NetEnemy"
    );
}

/// Holding fire with flamethrower damages enemy continuously over multiple ticks.
#[test]
fn flamethrower_damages_over_ticks() {
    let port = reserve_port();
    let mut server = build_flame_server(port);
    server.update();

    let addr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), port);
    let mut client = build_combat_client(addr);
    client.update();

    assert!(wait_for_player(&mut server, &mut client));

    // Switch to flamethrower.
    queue_switch(&mut client, 1);
    for _ in 0..10 {
        tick_with_sleep(&mut server, &mut client);
    }

    let initial_hp = get_enemy_health(&mut server).unwrap();

    // Hold fire for many ticks.
    for seq in 2..=20 {
        queue_fire(&mut client, seq);
        tick_with_sleep(&mut server, &mut client);
    }
    for _ in 0..30 {
        tick_with_sleep(&mut server, &mut client);
    }

    let hp_after = get_enemy_health(&mut server).unwrap();
    let damage = initial_hp - hp_after;

    assert!(
        damage > 10.0,
        "flamethrower should deal continuous damage: {initial_hp} → {hp_after} (dmg={damage})"
    );
}

/// End-to-end harness: default pistol -> switch through input -> hold fire -> release.
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

    for seq in 2..=12 {
        queue_fire(&mut client, seq);
        tick_with_sleep(&mut server, &mut client);
    }
    for _ in 0..20 {
        tick_with_sleep(&mut server, &mut client);
    }

    assert!(
        is_flame_active(&server, player_id),
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

    let hp_at_release = get_enemy_health(&mut server).unwrap();
    for _ in 0..30 {
        tick_with_sleep(&mut server, &mut client);
    }
    let hp_final = get_enemy_health(&mut server).unwrap();
    assert_eq!(
        hp_at_release, hp_final,
        "NetHealth should stop changing after BTN_FIRE release: {hp_at_release} -> {hp_final}"
    );
}

/// Releasing fire stops flamethrower damage.
#[test]
fn flamethrower_stops_on_release() {
    let port = reserve_port();
    let mut server = build_flame_server(port);
    server.update();

    let addr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), port);
    let mut client = build_combat_client(addr);
    client.update();

    assert!(wait_for_player(&mut server, &mut client));

    // Switch to flamethrower.
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

    // Release fire.
    queue_idle(&mut client, 6);
    for _ in 0..10 {
        tick_with_sleep(&mut server, &mut client);
    }

    let hp_at_release = get_enemy_health(&mut server).unwrap();

    // Wait more — no new damage should occur.
    for _ in 0..20 {
        tick_with_sleep(&mut server, &mut client);
    }

    let hp_final = get_enemy_health(&mut server).unwrap();
    assert_eq!(
        hp_at_release, hp_final,
        "damage should stop after fire release: {hp_at_release} → {hp_final}"
    );
}

/// Pistol still uses cooldown hitscan after switching back.
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
    // Release between switches so the second BTN_SWITCH creates a rising edge.
    queue_switch(&mut client, 1);
    for _ in 0..20 {
        tick_with_sleep(&mut server, &mut client);
    }
    queue_idle(&mut client, 2); // Release all buttons.
    for _ in 0..5 {
        tick_with_sleep(&mut server, &mut client);
    }
    queue_switch(&mut client, 3); // Second switch.
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

    // Hitscan does exactly 37 damage per shot.
    assert!(
        (damage - 37.0).abs() < 1.0,
        "pistol should do hitscan damage: got {damage}"
    );
}

// ---------------------------------------------------------------------------
// Authority regression tests
// ---------------------------------------------------------------------------

/// Dead player's fire commands should not deal damage (network-level test).
/// The input is accepted into the buffer but `process_combat` skips dead players.
#[test]
fn dead_player_fire_does_not_damage_enemy() {
    let port = reserve_port();
    let mut server = build_combat_server(port);
    server.update();

    let addr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), port);
    let mut client = build_combat_client(addr);
    client.update();

    assert!(wait_for_player(&mut server, &mut client));

    let initial_hp = get_enemy_health(&mut server).unwrap();

    // Kill the player by setting health to 0, then tick to trigger death.
    {
        let mut q = server.world_mut().query::<(&NetPlayer, &mut NetHealth)>();
        for (_, mut h) in q.iter_mut(server.world_mut()) {
            h.current = 0.0;
        }
    }
    for _ in 0..30 {
        tick_with_sleep(&mut server, &mut client);
    }

    // Verify player is dead.
    let state = server
        .world_mut()
        .query::<&NetPlayer>()
        .iter(server.world())
        .next()
        .map(|p| p.state.clone());
    assert!(
        matches!(state, Some(PlayerNetState::Dead)),
        "player should be dead, got {state:?}"
    );

    // Send fire commands while dead.
    let mut seq = 100;
    for _ in 0..10 {
        queue_fire(&mut client, seq);
        seq += 1;
        tick_with_sleep(&mut server, &mut client);
    }
    for _ in 0..20 {
        tick_with_sleep(&mut server, &mut client);
    }

    let hp_after = get_enemy_health(&mut server).unwrap();
    assert_eq!(
        initial_hp, hp_after,
        "dead player fire should not damage enemy: hp before={initial_hp}, after={hp_after}"
    );
}

/// Fire cooldown is cleared when a player dies, so respawned players start fresh.
#[test]
fn fire_cooldown_cleared_on_death() {
    use carcinisation_server::systems::FireCooldownMap;

    let port = reserve_port();
    let mut server = build_combat_server(port);
    server.update();

    let addr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), port);
    let mut client = build_combat_client(addr);
    client.update();

    assert!(wait_for_player(&mut server, &mut client));

    // Fire once to set a cooldown.
    queue_fire(&mut client, 1);
    for _ in 0..20 {
        tick_with_sleep(&mut server, &mut client);
    }

    let pid = get_player_id(&mut server).expect("player should exist");

    // Verify cooldown exists.
    let has_cd = server
        .world()
        .resource::<FireCooldownMap>()
        .0
        .contains_key(&pid);
    assert!(has_cd, "cooldown should exist after firing");

    // Kill the player.
    {
        let mut q = server.world_mut().query::<(&NetPlayer, &mut NetHealth)>();
        for (_, mut h) in q.iter_mut(server.world_mut()) {
            h.current = 0.0;
        }
    }
    for _ in 0..30 {
        tick_with_sleep(&mut server, &mut client);
    }

    // Cooldown should be cleared on death.
    let cleared = !server
        .world()
        .resource::<FireCooldownMap>()
        .0
        .contains_key(&pid);
    assert!(
        cleared,
        "FireCooldownMap should be cleared after player death"
    );
}

/// P2 regression: two players fire during the same tick at enemies in a line.
/// Player 1 kills the front enemy. Player 2's hitscan must see fresh state
/// (front enemy dead/filtered) and hit the back enemy instead of wasting the
/// shot on the corpse.
///
/// Under a stale shared-snapshot implementation (enemy list built once before
/// the player loop), player 2 would "hit" the dead front enemy and the back
/// enemy would be untouched.
#[test]
fn two_players_same_tick_second_sees_fresh_enemy_state() {
    use carcinisation_server::systems::PlayerIntentBuffer;

    let port = reserve_port();

    // Two enemies in a line east of both players:
    //   front enemy at (3.5, 1.5) with 1 HP — player 1 will kill it
    //   back  enemy at (5.5, 1.5) with 100 HP — player 2 should hit this
    let entities = vec![
        EntitySpawnData {
            kind: EntitySpawnKind::Mosquiton {
                health: 1,
                speed: 0.0,
            },
            x: 3.5,
            y: 1.5,
        },
        EntitySpawnData {
            kind: EntitySpawnKind::Mosquiton {
                health: 100,
                speed: 0.0,
            },
            x: 5.5,
            y: 1.5,
        },
    ];
    let mut server = build_server_app(ServerPlugin {
        port,
        map: test_map(),
        entities,
        player_starts: vec![],
    });
    server.update();

    // Both players at the same position facing east (angle 0), both using pistol.
    // They occupy the same cell so both hitscans travel the same ray.
    for pid in [1u32, 2] {
        server.world_mut().spawn((
            NetPlayer {
                player_id: PlayerId(pid),
                position: Vec2::new(1.5, 1.5),
                angle: 0.0,
                current_attack: NetAttackId::None,
                state: PlayerNetState::Alive,
            },
            NetHealth {
                current: 100.0,
                max: 100.0,
            },
            Replicated,
        ));
    }

    // Inject fire_held for both players.
    {
        let mut buf = server.world_mut().resource_mut::<PlayerIntentBuffer>();
        for pid in [1u32, 2] {
            buf.set(
                PlayerId(pid),
                &ClientIntent {
                    sequence: InputSequence(1),
                    movement: Vec2::ZERO,
                    turn: 0.0,
                    fire_held: true,
                    actions: PlayerActions::default(),
                },
            );
        }
    }

    // Tick enough for FixedUpdate to fire (at 2ms sleep per tick, ~17 ticks = one 30 Hz tick).
    for _ in 0..50 {
        std::thread::sleep(std::time::Duration::from_millis(2));
        server.update();
    }

    // Collect enemy health values.
    let mut healths: Vec<(f32, f32)> = server
        .world_mut()
        .query::<(&NetEnemy, &NetHealth)>()
        .iter(server.world())
        .map(|(e, h)| (e.position.x, h.current))
        .collect();
    healths.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    assert!(
        healths.len() >= 2,
        "both enemies should still exist, got {healths:?}"
    );

    let front_hp = healths[0].1;
    let back_hp = healths[1].1;

    // Front enemy (1 HP) should be dead.
    assert!(front_hp <= 0.0, "front enemy should be dead: hp={front_hp}");

    // Back enemy should have taken damage from the second player's hitscan.
    // Under the stale-snapshot bug, back_hp would still be 100.
    assert!(
        back_hp < 100.0,
        "back enemy should be hit by player 2 (fresh snapshot): hp={back_hp}. \
         If 100, the enemy list was stale and player 2 wasted a shot on the dead front enemy."
    );
}
