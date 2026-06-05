//! Shared helpers for networked combat integration tests.
//!
//! These tests require real UDP networking (client → server), so they use
//! `tick_with_sleep` rather than deterministic `FixedTimesteps`.

use std::net::SocketAddr;

use bevy::prelude::*;
use bevy_replicon::prelude::ClientTriggerExt;
use carcinisation_fps_core::map::{EntitySpawnData, EntitySpawnKind, Map, test_map};
use carcinisation_net::components::NetEnemy;
use carcinisation_net::{
    ClientIntent, FlameActive, InputSequence, NetAttackId, NetEnemyState, NetHealth, NetPlayer,
    NetProtocolPlugin, PlayerActions, PlayerId, register_net_all,
};
use carcinisation_server::ServerPlugin;
use carcinisation_server::systems::FlameActiveTracker;

use super::{build_client_app, build_server_app, tick_with_sleep};

// ---------------------------------------------------------------------------
// Resources & systems
// ---------------------------------------------------------------------------

#[derive(Resource, Default)]
pub struct TestIntentQueue(pub Vec<ClientIntent>);

#[derive(Resource, Default)]
pub struct ReceivedFlameEvents(pub Vec<FlameActive>);

pub fn send_queued_intents(mut commands: Commands, mut queue: ResMut<TestIntentQueue>) {
    for intent in queue.0.drain(..) {
        commands.client_trigger(intent);
    }
}

#[allow(clippy::needless_pass_by_value)] // Bevy observer signature requires `On<T>` by value.
pub fn capture_flame_active(trigger: On<FlameActive>, mut events: ResMut<ReceivedFlameEvents>) {
    events.0.push(trigger.event().clone());
}

// ---------------------------------------------------------------------------
// Intent queue helpers
// ---------------------------------------------------------------------------

pub fn queue_fire(app: &mut App, seq: u32) {
    app.world_mut()
        .resource_mut::<TestIntentQueue>()
        .0
        .push(ClientIntent {
            sequence: InputSequence(seq),
            movement: Vec2::ZERO,
            turn: 0.0,
            fire_held: true,
            aim_held: false,
            aim_offset: 0.0,
            actions: PlayerActions::default(),
        });
}

pub fn queue_idle(app: &mut App, seq: u32) {
    app.world_mut()
        .resource_mut::<TestIntentQueue>()
        .0
        .push(ClientIntent::idle(InputSequence(seq)));
}

pub fn queue_switch(app: &mut App, seq: u32) {
    app.world_mut()
        .resource_mut::<TestIntentQueue>()
        .0
        .push(ClientIntent {
            sequence: InputSequence(seq),
            movement: Vec2::ZERO,
            turn: 0.0,
            fire_held: false,
            aim_held: false,
            aim_offset: 0.0,
            actions: PlayerActions::from_raw(PlayerActions::WEAPON_SWITCH),
        });
}

// ---------------------------------------------------------------------------
// Server/client builders
// ---------------------------------------------------------------------------

/// Build a server with `test_map` + one enemy at (4.5, 1.5) — directly east of
/// spawn (1.5, 1.5) at angle 0.
pub fn build_combat_server(port: u16) -> App {
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
        admin_socket: None,
        instance_name: "test".to_string(),
        map_path: "test_map".to_string(),
    })
}

/// Build a server with an enemy at (3.5, 1.5) — close enough for flamethrower
/// range (5.0 units) from spawn (1.5, 1.5) at angle 0.
pub fn build_flame_server(port: u16) -> App {
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
        admin_socket: None,
        instance_name: "test".to_string(),
        map_path: "test_map".to_string(),
    })
}

pub fn build_combat_client(addr: SocketAddr) -> App {
    let mut app = build_client_app(NetProtocolPlugin, register_net_all, addr);
    app.init_resource::<TestIntentQueue>();
    app.init_resource::<ReceivedFlameEvents>();
    app.add_observer(capture_flame_active);
    app.add_systems(Update, send_queued_intents);
    app
}

pub fn load_default_map_data() -> carcinisation_fps_core::map::MapLoadData {
    let ron = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/config/fp/test_room.fp_map.ron"
    ))
    .expect("default multiplayer map should exist");
    Map::load_data(&ron).expect("default multiplayer map should parse")
}

pub fn build_default_map_server(port: u16) -> App {
    let map_data = load_default_map_data();
    build_server_app(ServerPlugin {
        port,
        map: map_data.map,
        entities: map_data.entities,
        player_starts: map_data.player_starts,
        admin_socket: None,
        instance_name: "test".to_string(),
        map_path: "test_map".to_string(),
    })
}

pub fn open_test_map(width: usize, height: usize) -> Map {
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

// ---------------------------------------------------------------------------
// Query helpers
// ---------------------------------------------------------------------------

pub fn wait_for_player(server: &mut App, client: &mut App) -> bool {
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

pub fn get_player_id(server: &mut App) -> Option<PlayerId> {
    server
        .world_mut()
        .query::<&NetPlayer>()
        .iter(server.world())
        .next()
        .map(|p| p.player_id)
}

pub fn get_player_attack(server: &mut App) -> Option<NetAttackId> {
    server
        .world_mut()
        .query::<&NetPlayer>()
        .iter(server.world())
        .next()
        .map(|p| p.current_attack)
}

pub fn is_flame_active(server: &App, player_id: PlayerId) -> bool {
    server
        .world()
        .resource::<FlameActiveTracker>()
        .0
        .get(&player_id)
        .copied()
        .unwrap_or(false)
}

pub fn client_received_flame_event(client: &App, player_id: PlayerId, active: bool) -> bool {
    client
        .world()
        .resource::<ReceivedFlameEvents>()
        .0
        .iter()
        .any(|event| event.player_id == player_id && event.active == active)
}

pub fn enemy_count(server: &mut App) -> usize {
    server
        .world_mut()
        .query::<&NetEnemy>()
        .iter(server.world())
        .count()
}

pub fn any_enemy_damaged(server: &mut App) -> bool {
    server
        .world_mut()
        .query::<(&NetEnemy, &NetHealth)>()
        .iter(server.world())
        .any(|(_, health)| health.current < health.max)
}

pub fn enemy_positions_by_object_id(server: &mut App) -> Vec<(u32, Vec2, NetEnemyState)> {
    let mut positions: Vec<_> = server
        .world_mut()
        .query::<&NetEnemy>()
        .iter(server.world())
        .map(|enemy| (enemy.object_id.0, enemy.position, enemy.state))
        .collect();
    positions.sort_by_key(|(object_id, _, _)| *object_id);
    positions
}

pub fn wait_for_client_enemy_position(server: &mut App, client: &mut App) -> Option<Vec2> {
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
