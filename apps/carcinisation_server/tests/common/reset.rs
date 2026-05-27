//! Shared helpers for map-reset and admin-socket integration tests.

use std::time::Duration;

use bevy::prelude::*;
use carcinisation_fps_core::map::{EntitySpawnData, EntitySpawnKind, test_map};
use carcinisation_fps_core::pickup::PickupKind;
use carcinisation_net::{NetAttackId, NetHealth, NetPlayer, PlayerId, PlayerNetState};
use carcinisation_server::ServerPlugin;
use carcinisation_server::systems::ServerQuickTurn;

use super::{build_server_app, reserve_port};

// ---------------------------------------------------------------------------
// Test data factories
// ---------------------------------------------------------------------------

pub fn one_enemy() -> Vec<EntitySpawnData> {
    vec![EntitySpawnData {
        x: 3.5,
        y: 1.5,
        kind: EntitySpawnKind::Mosquiton {
            health: 100,
            speed: 0.0,
        },
    }]
}

pub fn two_pickups() -> Vec<EntitySpawnData> {
    vec![
        EntitySpawnData {
            x: 2.0,
            y: 2.0,
            kind: EntitySpawnKind::Pickup {
                kind: PickupKind::Health,
                respawnable: true,
            },
        },
        EntitySpawnData {
            x: 5.0,
            y: 5.0,
            kind: EntitySpawnKind::Pickup {
                kind: PickupKind::Health,
                respawnable: false,
            },
        },
    ]
}

// ---------------------------------------------------------------------------
// Server builders
// ---------------------------------------------------------------------------

pub fn build_reset_server(entities: Vec<EntitySpawnData>, admin_socket: Option<String>) -> App {
    let port = reserve_port();
    build_server_app(ServerPlugin {
        port,
        map: test_map(),
        entities,
        player_starts: vec![],
        admin_socket,
        instance_name: "test".to_string(),
        map_path: "test_map".to_string(),
    })
}

// ---------------------------------------------------------------------------
// Entity helpers
// ---------------------------------------------------------------------------

/// Spawn an alive player with full health and `ServerQuickTurn`. Returns the entity.
/// Unlike `common::spawn_alive_player`, this omits `Replicated` (server-only tests)
/// and includes `ServerQuickTurn` (needed by reset logic).
pub fn spawn_player(app: &mut App, pid: u32, x: f32, y: f32) -> Entity {
    app.world_mut()
        .spawn((
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
            ServerQuickTurn::default(),
        ))
        .id()
}

// ---------------------------------------------------------------------------
// Tick / query helpers
// ---------------------------------------------------------------------------

pub fn tick_server_n(app: &mut App, ticks: u32) {
    for _ in 0..ticks {
        std::thread::sleep(Duration::from_millis(2));
        app.update();
    }
}

pub fn count<C: Component>(app: &mut App) -> usize {
    app.world_mut()
        .query_filtered::<Entity, With<C>>()
        .iter(app.world())
        .count()
}
