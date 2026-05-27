//! Server-side map reset.
//!
//! Despawns gameplay entities, respawns map entities, and resets players to
//! spawn points with full health. Connected clients are preserved — they see
//! the world reinitialised around them.

use bevy::prelude::*;
use bevy_replicon::prelude::*;
use carcinisation_net::{
    FlameActive, NetAttackId, NetHealth, NetPickup, NetPlayer, PlayerNetState,
};

use super::enemy_attack::PendingProjectile;
use super::{
    BurnContactCooldowns, FireCooldownMap, FlameActiveTracker, FlameCharCooldowns, NetEnemy,
    NetProjectile, NextProjectileId, ServerQuickTurn,
};
use crate::{MapEntities, MapPlayerStarts, SpawnIndex, spawn_map_enemies_inner};

/// Resource flag set by the admin socket handler. Cleared after the reset
/// system processes it.
#[derive(Resource, Default)]
pub struct MapResetRequested(pub bool);

/// Despawn all map gameplay entities and projectiles, reset players to spawn points,
/// re-spawn map entities from the map definition, and clear transient combat state.
#[allow(clippy::too_many_arguments)]
pub fn handle_map_reset(
    mut reset: ResMut<MapResetRequested>,
    mut commands: Commands,
    enemies: Query<Entity, With<NetEnemy>>,
    pickups: Query<Entity, With<NetPickup>>,
    projectiles: Query<Entity, With<NetProjectile>>,
    pending_projectiles: Query<Entity, With<PendingProjectile>>,
    mut players: Query<(Entity, &mut NetPlayer, &mut NetHealth)>,
    player_starts: Res<MapPlayerStarts>,
    map_entities: Res<MapEntities>,
    mut spawn_idx: ResMut<SpawnIndex>,
    mut next_proj_id: ResMut<NextProjectileId>,
    mut cooldowns: ResMut<FireCooldownMap>,
    mut flame_tracker: ResMut<FlameActiveTracker>,
    mut char_cooldowns: ResMut<FlameCharCooldowns>,
    mut burn_cooldowns: ResMut<BurnContactCooldowns>,
) {
    if !reset.0 {
        return;
    }
    reset.0 = false;

    // --- Despawn all enemies (alive, dying, dead) ---
    let mut enemy_count = 0u32;
    for entity in enemies.iter() {
        commands.entity(entity).despawn();
        enemy_count += 1;
    }

    // --- Despawn all pickups before re-spawning map entities ---
    let mut pickup_count = 0u32;
    for entity in pickups.iter() {
        commands.entity(entity).despawn();
        pickup_count += 1;
    }

    // --- Despawn all projectiles (live + pending) ---
    let mut proj_count = 0u32;
    for entity in projectiles.iter() {
        commands.entity(entity).despawn();
        proj_count += 1;
    }
    for entity in pending_projectiles.iter() {
        commands.entity(entity).despawn();
    }

    // --- Reset players to spawn points ---
    spawn_idx.0 = 0;
    let mut player_count = 0u32;
    for (entity, mut np, mut health) in &mut players {
        let spawn = player_starts.0[spawn_idx.0 % player_starts.0.len()];
        np.position = Vec2::new(spawn.x, spawn.y);
        np.angle = spawn.angle_deg.to_radians();
        np.state = PlayerNetState::Alive;
        np.current_attack = NetAttackId::None;
        np.flame_active = false;
        health.current = health.max;
        // Remove RespawnTimer if the player was dead.
        commands
            .entity(entity)
            .remove::<super::RespawnTimer>()
            .insert(ServerQuickTurn::default());
        spawn_idx.0 += 1;
        player_count += 1;
    }

    // --- Clear transient combat resources ---
    // Player input/intent buffers are NOT cleared — players remain connected
    // and staleness handling naturally zeroes movement.
    cooldowns.0.clear();

    // Notify clients to stop rendering flame effects before clearing tracker.
    for (&player_id, &active) in &flame_tracker.0 {
        if active {
            commands.server_trigger(ToClients {
                mode: SendMode::Broadcast,
                message: FlameActive {
                    player_id,
                    active: false,
                },
            });
        }
    }
    flame_tracker.0.clear();
    char_cooldowns.0.clear();
    burn_cooldowns.0.clear();
    next_proj_id.0 = 0;

    // --- Re-spawn map entities from map definition ---
    let respawned = spawn_map_enemies_inner(&mut commands, &map_entities.0);

    info!(
        "Map reset: despawned {enemy_count} enemies + {pickup_count} pickups + \
         {proj_count} projectiles, reset {player_count} players, respawned \
         {respawned} map entities"
    );
}
