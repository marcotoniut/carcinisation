//! Server-side soft body-occupancy separation.
//!
//! Entities with [`OccupiesSpace`] are gently pushed apart when their XZ
//! circles and vertical Y ranges overlap. Enemies yield more than players
//! so the player can squeeze through tight spaces.
//!
//! Separation is **server-authoritative and not client-predicted**. Player
//! displacement from separation appears in the next `InputAck` (same timing
//! as Spidey lunge one-shot push).

use bevy::prelude::*;
use carcinisation_fps_core::config::{FpsCombatConfig, FpsMovementConfig};
use carcinisation_fps_core::occupancy::{
    OccupancyEntry, OccupancyMode, OccupancyProfile, OccupancyVolume, compute_separation,
};
use carcinisation_fps_core::try_move;
use carcinisation_net::{NetEnemy, NetEnemyState, NetPlayer, PlayerNetState};

use carcinisation_fps_core::occupancy::OccupancyImpulse;

use crate::ServerMap;

/// Server-side push impulse on a player entity (e.g. Spidey lunge knockback).
///
/// Ticked each frame in `MovementSet` before `send_input_acks`. The impulse
/// state is replicated via `InputAck` fields so the client can replay the
/// same decay during prediction.
///
/// **Upgrade path**: when a `PredictionEvent` system is built, this component
/// and its `InputAck` fields should migrate to event-based delivery.
#[derive(Component, Debug, Clone)]
pub struct ServerPlayerImpulse(pub OccupancyImpulse);

/// Tick active player impulses: apply displacement via `try_move`, remove
/// when expired. Runs in `MovementSet` after `apply_buffered_movement` and
/// before `send_input_acks`.
pub fn tick_player_impulses(
    mut commands: Commands,
    mut players: Query<(Entity, &mut NetPlayer, &ServerPlayerImpulse)>,
    server_map: Res<ServerMap>,
    fixed_time: Res<Time<Fixed>>,
    movement_config: Res<FpsMovementConfig>,
) {
    let dt = fixed_time.delta_secs();
    for (entity, mut player, impulse_comp) in &mut players {
        if !matches!(player.state, PlayerNetState::Alive) {
            commands.entity(entity).remove::<ServerPlayerImpulse>();
            continue;
        }
        let mut impulse = impulse_comp.0;
        let displacement = impulse.tick(dt);
        if displacement != Vec2::ZERO {
            try_move(
                &mut player.position,
                displacement,
                movement_config.collision_margin,
                &server_map.0,
            );
        }
        if impulse.is_expired() {
            commands.entity(entity).remove::<ServerPlayerImpulse>();
        } else {
            commands.entity(entity).insert(ServerPlayerImpulse(impulse));
        }
    }
}

/// System set for occupancy resolution.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct OccupancySet;

/// Server-side occupancy component. Attached to entities that participate in
/// soft body separation.
#[derive(Component, Debug, Clone)]
pub struct OccupiesSpace {
    pub volume: OccupancyVolume,
    pub profile: OccupancyProfile,
    pub weight: f32,
    pub pushable: bool,
    pub separation_strength: f32,
    /// Wall collision margin used by `try_move` for this entity.
    pub collision_margin: f32,
}

/// Snapshot of one entity's occupancy + position for the separation pass.
/// Sorted by a stable key for determinism.
struct OccupancySnapshot {
    entity: Entity,
    is_player: bool,
    entry: OccupancyEntry,
    collision_margin: f32,
}

/// Resolve soft occupancy separation for all entities with [`OccupiesSpace`].
///
/// Gathers snapshots in stable deterministic order (sorted by `Entity` index),
/// computes separation displacement for each pushable entity, and applies it
/// via `try_move` for wall safety.
///
/// `stable_index` is derived from `Entity::to_bits()`, which is deterministic
/// within a server run given deterministic spawn order. It is **not** globally
/// stable across separate runs or platforms — but this is sufficient because
/// separation only needs intra-tick consistency, not cross-run reproducibility.
///
/// Runs in `OccupancySet` after `EnemyAttackSet` (so lunge one-shot push is
/// already applied). Player separation appears in the next tick's `InputAck`.
#[allow(clippy::too_many_arguments, clippy::cast_possible_truncation)]
pub fn resolve_soft_occupancy(
    mut players: Query<(Entity, &mut NetPlayer, &OccupiesSpace), Without<NetEnemy>>,
    mut enemies: Query<(Entity, &mut NetEnemy, &OccupiesSpace), Without<NetPlayer>>,
    server_map: Res<ServerMap>,
    combat_config: Res<FpsCombatConfig>,
) {
    let max_step = combat_config.occupancy.max_separation_step;

    // Build sorted snapshot of all occupancy entities.
    let mut snapshots: Vec<OccupancySnapshot> = Vec::new();

    for (entity, player, occ) in &players {
        if !matches!(player.state, PlayerNetState::Alive) {
            continue;
        }
        snapshots.push(OccupancySnapshot {
            entity,
            is_player: true,
            entry: OccupancyEntry {
                position: player.position,
                height_offset: 0.0,
                volume: occ.volume,
                mode: occ.profile.to_mode(),
                weight: occ.weight,
                pushable: occ.pushable,
                separation_strength: occ.separation_strength,
                stable_index: entity.to_bits() as u32,
            },
            collision_margin: occ.collision_margin,
        });
    }

    for (entity, enemy, occ) in &enemies {
        if matches!(
            enemy.state,
            NetEnemyState::Dying { .. } | NetEnemyState::Dead { .. }
        ) {
            continue;
        }
        snapshots.push(OccupancySnapshot {
            entity,
            is_player: false,
            entry: OccupancyEntry {
                position: enemy.position,
                height_offset: enemy.visual_height,
                volume: occ.volume,
                mode: occ.profile.to_mode(),
                weight: occ.weight,
                pushable: occ.pushable,
                separation_strength: occ.separation_strength,
                stable_index: entity.to_bits() as u32,
            },
            collision_margin: occ.collision_margin,
        });
    }

    // Sort by Entity index for deterministic ordering.
    snapshots.sort_by_key(|s| s.entity);

    // Build the flat entries slice for compute_separation.
    let entries: Vec<OccupancyEntry> = snapshots.iter().map(|s| s.entry).collect();

    // Compute and apply separation for each pushable entity.
    for (i, snapshot) in snapshots.iter().enumerate() {
        if !snapshot.entry.pushable || snapshot.entry.mode == OccupancyMode::Disabled {
            continue;
        }

        let displacement = compute_separation(i, &entries, max_step);
        if displacement == Vec2::ZERO {
            continue;
        }

        if snapshot.is_player {
            if let Ok((_, mut player, _)) = players.get_mut(snapshot.entity) {
                try_move(
                    &mut player.position,
                    displacement,
                    snapshot.collision_margin,
                    &server_map.0,
                );
            }
        } else if let Ok((_, mut enemy, _)) = enemies.get_mut(snapshot.entity) {
            try_move(
                &mut enemy.position,
                displacement,
                snapshot.collision_margin,
                &server_map.0,
            );
        }
    }
}

/// Sync [`OccupiesSpace`] profile from enemy state.
///
/// Updates the occupancy profile based on the enemy's current `NetEnemyState`
/// and (for Spideys) the sim state, so that dying/dead enemies stop
/// participating in separation and lunging Spideys get `Airborne` mode.
pub fn sync_enemy_occupancy_profiles(
    mut enemies: Query<
        (
            &NetEnemy,
            &mut OccupiesSpace,
            Option<&super::enemy_attack::ServerSpideySim>,
        ),
        Without<NetPlayer>,
    >,
) {
    for (enemy, mut occ, spidey_sim) in &mut enemies {
        occ.profile = match enemy.state {
            NetEnemyState::Dying { .. } | NetEnemyState::Dead { .. } => OccupancyProfile::Disabled,
            _ => {
                // Check Spidey-specific states for airborne profiles.
                use carcinisation_fps_core::spidey::SpideySimState;
                spidey_sim.map_or(OccupancyProfile::Standing, |sim| match &sim.sim_state {
                    SpideySimState::LungeAttack { .. } | SpideySimState::LungeWindup { .. } => {
                        OccupancyProfile::Lunging
                    }
                    SpideySimState::HopMove { .. } => OccupancyProfile::Airborne,
                    _ => OccupancyProfile::Standing,
                })
            }
        };
    }
}

/// Build an [`OccupiesSpace`] component for a player entity.
#[must_use]
pub const fn player_occupancy(
    config: &FpsCombatConfig,
    movement_config: &FpsMovementConfig,
) -> OccupiesSpace {
    let occ = &config.occupancy;
    OccupiesSpace {
        volume: OccupancyVolume {
            radius_xz: movement_config.collision_margin,
            y_min: 0.0,
            y_max: occ.body_height,
        },
        profile: OccupancyProfile::Standing,
        weight: occ.player_weight,
        pushable: true,
        separation_strength: occ.player_separation_strength,
        collision_margin: movement_config.collision_margin,
    }
}

/// Build an [`OccupiesSpace`] component for an enemy entity.
#[must_use]
pub const fn enemy_occupancy(config: &FpsCombatConfig, collision_radius: f32) -> OccupiesSpace {
    let occ = &config.occupancy;
    OccupiesSpace {
        volume: OccupancyVolume {
            radius_xz: collision_radius,
            y_min: 0.0,
            y_max: occ.body_height,
        },
        profile: OccupancyProfile::Standing,
        weight: occ.enemy_weight,
        pushable: true,
        separation_strength: occ.enemy_separation_strength,
        collision_margin: collision_radius,
    }
}
