//! Server-authoritative player death and respawn.
//!
//! Runs in `CombatSet` (after projectile damage has been applied).
//! - When `NetHealth.current <= 0` and player is `Alive`: transition to `Dead`,
//!   send `DeathEffect`, reset position.
//! - When `Dead { respawn_timer }` ticks down to 0: respawn at a spawn point
//!   with full health.

use crate::MapPlayerStarts;
use crate::systems::{
    BurnContactCooldowns, FireCooldownMap, FlameActiveTracker, FlameCharCooldowns,
};
use bevy::prelude::*;
use bevy_replicon::prelude::*;
use carcinisation_fps_core::config::PLAYER_RESPAWN_DELAY_SECS;
use carcinisation_net::{
    DeathEffect, FlameActive, NetHealth, NetPlayer, NetworkObjectId, PlayerNetState,
};

/// Check for player death and handle respawn timers.
#[allow(clippy::too_many_arguments)]
pub fn tick_player_lifecycle(
    mut commands: Commands,
    mut players: Query<(&mut NetPlayer, &mut NetHealth)>,
    player_starts: Res<MapPlayerStarts>,
    fixed_time: Res<Time<Fixed>>,
    mut respawn_idx: Local<usize>,
    mut cooldowns: ResMut<FireCooldownMap>,
    mut flame_tracker: ResMut<FlameActiveTracker>,
    mut char_cooldowns: ResMut<FlameCharCooldowns>,
    mut burn_cooldowns: ResMut<BurnContactCooldowns>,
) {
    let dt = fixed_time.delta_secs();

    for (mut player, mut health) in &mut players {
        match &player.state {
            PlayerNetState::Alive => {
                if health.current > 0.0 {
                    continue;
                }

                // Player just died.
                player.state = PlayerNetState::Dead {
                    respawn_timer: PLAYER_RESPAWN_DELAY_SECS,
                };

                // Clear per-player combat state so respawn starts clean.
                cooldowns.remove_player(&player.player_id);
                char_cooldowns.remove_player(&player.player_id);
                burn_cooldowns.remove_player(&player.player_id);
                if flame_tracker
                    .0
                    .get(&player.player_id)
                    .copied()
                    .unwrap_or(false)
                {
                    flame_tracker.0.insert(player.player_id, false);
                    commands.server_trigger(ToClients {
                        mode: SendMode::Broadcast,
                        message: FlameActive {
                            player_id: player.player_id,
                            active: false,
                        },
                    });
                }

                commands.server_trigger(ToClients {
                    mode: SendMode::Broadcast,
                    message: DeathEffect {
                        target_id: NetworkObjectId(player.player_id.0),
                        was_player: true,
                    },
                });

                info!(
                    "Player {:?} died, respawning in {PLAYER_RESPAWN_DELAY_SECS}s",
                    player.player_id
                );
            }
            PlayerNetState::Dead { respawn_timer } => {
                let new_timer = respawn_timer - dt;
                if new_timer > 0.0 {
                    player.state = PlayerNetState::Dead {
                        respawn_timer: new_timer,
                    };
                    continue;
                }

                // Respawn.
                let spawn = player_starts.0[*respawn_idx % player_starts.0.len()];
                *respawn_idx += 1;

                player.position = Vec2::new(spawn.x, spawn.y);
                player.angle = spawn.angle_deg.to_radians();
                player.state = PlayerNetState::Alive;
                health.current = health.max;

                info!(
                    "Player {:?} respawned at ({:.1}, {:.1})",
                    player.player_id, spawn.x, spawn.y
                );
            }
        }
    }
}
