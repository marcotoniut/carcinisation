//! Server-authoritative Mosquiton attacks via shared `fps_core` simulation.
//!
//! Uses `tick_mosquiton_sim` from `carcinisation_fps_core` for the full
//! Mosquiton combat state machine (movement, cooldowns, melee/ranged decisions).
//! The server is an ECS adapter: it converts replicated components to/from
//! the portable sim model and maps outputs to network effects.

use bevy::prelude::*;
use bevy_replicon::prelude::*;
use carcinisation_net::{
    DamageEffect, EnemyAttackKind, EnemyAttackVisual, NetEnemy, NetEnemyState, NetEnemyType,
    NetHealth, NetPlayer, NetProjectileType, NetworkObjectId, Owner, PlayerId, PlayerNetState,
};

use carcinisation_fps_core::config::FpsCombatConfig;
use carcinisation_fps_core::mosquiton::{
    MosquitonSim, MosquitonSimConfig, MosquitonSimState, tick_mosquiton_sim,
};

use crate::ServerMap;

use super::NetProjectile;

/// Per-enemy Mosquiton simulation state, attached at spawn time.
///
/// Stores the `MosquitonSim` fields that persist across ticks.
/// The sim config is shared via `ServerMosquitonSimConfig`.
#[derive(Component, Debug, Clone)]
pub struct ServerMosquitonSim {
    pub shoot_cooldown: f32,
    pub melee_cooldown: f32,
    pub decision_timer: f32,
    pub shoot_anim_elapsed: Option<f32>,
    pub sim_state: MosquitonSimState,
    /// Stable per-instance seed for deterministic sim decisions.
    pub seed: u32,
}

impl Default for ServerMosquitonSim {
    fn default() -> Self {
        Self {
            shoot_cooldown: 0.0,
            melee_cooldown: 0.0,
            decision_timer: 0.0,
            shoot_anim_elapsed: None,
            sim_state: MosquitonSimState::Pursue,
            seed: 0,
        }
    }
}

/// Per-enemy sim config, attached at spawn time.
/// Allows map-authored speed overrides.
#[derive(Component, Debug, Clone, Default)]
pub struct ServerMosquitonSimConfig(pub MosquitonSimConfig);

impl ServerMosquitonSimConfig {
    #[must_use]
    pub fn with_speed(speed: f32) -> Self {
        Self(MosquitonSimConfig {
            move_speed: speed,
            ..MosquitonSimConfig::default()
        })
    }
}

/// Pending ranged projectile — delays spawn so the shoot animation leads.
/// Cancelled if the source enemy dies before the timer expires.
#[derive(Component, Debug)]
pub struct PendingProjectile {
    pub timer: f32,
    pub source_entity: Entity,
    pub position: Vec2,
    pub angle: f32,
    pub damage: f32,
    pub object_id: NetworkObjectId,
}

/// System set for enemy attack spawning.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct EnemyAttackSet;

/// Global counter for projectile `NetworkObjectId`s.
#[derive(Resource, Default)]
pub struct NextProjectileId(pub u32);

impl NextProjectileId {
    pub fn allocate(&mut self) -> NetworkObjectId {
        self.0 += 1;
        // Offset by 10000 to avoid collision with enemy object IDs.
        NetworkObjectId(self.0 + 10000)
    }
}

/// Deferred melee hit to apply after the enemy iteration.
struct MeleeHit {
    target_player_id: PlayerId,
    damage: f32,
}

/// Tick Mosquiton enemies using the shared `fps_core` simulation.
///
/// Handles movement, attack cooldowns, and attack decisions in one call
/// via `tick_mosquiton_sim`. Maps outputs to ECS/network effects.
#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::cast_precision_loss
)]
pub fn tick_enemy_attacks(
    mut commands: Commands,
    mut enemies: Query<(
        Entity,
        &mut NetEnemy,
        &NetHealth,
        &mut ServerMosquitonSim,
        &ServerMosquitonSimConfig,
    )>,
    players: Query<&NetPlayer>,
    mut player_health: Query<(&NetPlayer, &mut NetHealth), Without<NetEnemy>>,
    fixed_time: Res<Time<Fixed>>,
    mut next_id: ResMut<NextProjectileId>,
    server_map: Res<ServerMap>,
    combat_config: Res<FpsCombatConfig>,
) {
    let dt = fixed_time.delta_secs();
    let mut melee_hits: Vec<MeleeHit> = Vec::new();

    for (enemy_entity, mut enemy, health, mut mosquiton_sim, sim_config) in &mut enemies {
        if enemy.enemy_type != NetEnemyType::Mosquiton {
            continue;
        }

        // Skip dead/dying enemies.
        if health.current <= 0.0
            || matches!(
                enemy.state,
                NetEnemyState::Dying { .. } | NetEnemyState::Dead { .. }
            )
        {
            continue;
        }

        // Find nearest alive player per-enemy (not shared).
        let Some(player_pos) = nearest_alive_player_pos(enemy.position, &players) else {
            continue;
        };

        // Build sim from per-entity state + shared NetEnemy position.
        let mut sim = MosquitonSim {
            position: enemy.position,
            state: mosquiton_sim.sim_state.clone(),
            shoot_cooldown: mosquiton_sim.shoot_cooldown,
            melee_cooldown: mosquiton_sim.melee_cooldown,
            decision_timer: mosquiton_sim.decision_timer,
            shoot_anim_elapsed: mosquiton_sim.shoot_anim_elapsed,
            seed: mosquiton_sim.seed,
        };

        let output = tick_mosquiton_sim(&mut sim, &sim_config.0, player_pos, &server_map.0, dt);

        // Write back sim state.
        enemy.position = sim.position;
        mosquiton_sim.sim_state = sim.state.clone();
        mosquiton_sim.shoot_cooldown = sim.shoot_cooldown;
        mosquiton_sim.melee_cooldown = sim.melee_cooldown;
        mosquiton_sim.decision_timer = sim.decision_timer;
        mosquiton_sim.shoot_anim_elapsed = sim.shoot_anim_elapsed;

        // Map sim state → NetEnemyState for replication.
        let net_state = sim_state_to_net(&sim.state);
        if !matches!(
            enemy.state,
            NetEnemyState::Dying { .. } | NetEnemyState::Dead { .. }
        ) {
            enemy.state = net_state;
        }

        // Handle shoot animation start → visual event + deferred projectile.
        if output.started_shoot_anim {
            commands.server_trigger(ToClients {
                mode: SendMode::Broadcast,
                message: EnemyAttackVisual {
                    object_id: enemy.object_id,
                    kind: EnemyAttackKind::Ranged,
                },
            });

            let to_target = player_pos - enemy.position;
            let dist = to_target.length();
            if dist > 0.01 {
                let dir = to_target / dist;
                let angle = dir.y.atan2(dir.x);
                let object_id = next_id.allocate();

                commands.spawn(PendingProjectile {
                    timer: combat_config.mosquiton_shoot_cue_secs,
                    source_entity: enemy_entity,
                    position: enemy.position,
                    angle,
                    damage: sim_config.0.blood_shot_damage as f32,
                    object_id,
                });

                debug!(
                    "Enemy {:?} queued projectile {:?} at player angle={:.2}",
                    enemy.object_id, object_id, angle
                );
            }
        }

        // Handle melee start → visual event + deferred damage.
        if output.started_melee {
            commands.server_trigger(ToClients {
                mode: SendMode::Broadcast,
                message: EnemyAttackVisual {
                    object_id: enemy.object_id,
                    kind: EnemyAttackKind::Melee,
                },
            });
        }

        // Handle melee damage output.
        if let Some((damage, _source)) = output.melee_damage
            && let Some(target_pid) = nearest_alive_player_id(enemy.position, &players)
        {
            melee_hits.push(MeleeHit {
                target_player_id: target_pid,
                damage: damage as f32,
            });
            debug!(
                "Enemy {:?} melee hit player {:?}",
                enemy.object_id, target_pid
            );
        }

        // Note: `output.projectile` is produced by the sim when shoot_anim_elapsed
        // reaches shoot_cue_secs. On the server, we use PendingProjectile instead
        // (deferred spawn for animation lead). The sim's projectile output is ignored;
        // the deferred spawn handles it.
    }

    // Apply deferred melee damage.
    for hit in melee_hits {
        for (player, mut health) in &mut player_health {
            if player.player_id == hit.target_player_id
                && matches!(player.state, PlayerNetState::Alive)
                && health.current > 0.0
            {
                health.current = (health.current - hit.damage).max(0.0);
                commands.server_trigger(ToClients {
                    mode: SendMode::Broadcast,
                    message: DamageEffect {
                        target_id: NetworkObjectId(player.player_id.0),
                        damage: hit.damage,
                        remaining_health: health.current,
                        was_player: true,
                    },
                });
                break;
            }
        }
    }
}

/// Spawn deferred projectiles after shoot lead time expires.
/// Cancels if the source enemy died during the delay.
pub fn tick_pending_projectiles(
    mut commands: Commands,
    mut pending: Query<(Entity, &mut PendingProjectile)>,
    enemies: Query<&NetEnemy>,
    fixed_time: Res<Time<Fixed>>,
    combat_config: Res<FpsCombatConfig>,
) {
    let dt = fixed_time.delta_secs();
    for (entity, mut p) in &mut pending {
        // Cancel if source enemy died or was despawned.
        let source_alive = enemies.get(p.source_entity).is_ok_and(|e| {
            !matches!(
                e.state,
                NetEnemyState::Dying { .. } | NetEnemyState::Dead { .. }
            )
        });
        if !source_alive {
            commands.entity(entity).despawn();
            continue;
        }

        p.timer -= dt;
        if p.timer <= 0.0 {
            commands.spawn((
                NetProjectile {
                    object_id: p.object_id,
                    position: p.position,
                    angle: p.angle,
                    owner: Owner(PlayerId(0)),
                    damage: p.damage,
                    projectile_type: NetProjectileType::BloodShot,
                },
                super::projectile::ProjectileTtl(combat_config.projectile_lifetime),
                Replicated,
            ));
            commands.entity(entity).despawn();
        }
    }
}

/// Map `MosquitonSimState` → `NetEnemyState` for replication.
fn sim_state_to_net(state: &MosquitonSimState) -> NetEnemyState {
    match state {
        MosquitonSimState::Pursue => NetEnemyState::Chase,
        MosquitonSimState::RangedAttack { .. }
        | MosquitonSimState::MeleeAttack { .. }
        | MosquitonSimState::Recover { .. } => NetEnemyState::HoldingRange,
        MosquitonSimState::Dying { .. } => NetEnemyState::Dying { burn: false },
        MosquitonSimState::BurningCorpse { .. } => NetEnemyState::Dying { burn: true },
        MosquitonSimState::Dead => NetEnemyState::Dead { burn: false },
    }
}

fn nearest_alive_player_pos(enemy_pos: Vec2, players: &Query<&NetPlayer>) -> Option<Vec2> {
    players
        .iter()
        .filter(|p| matches!(p.state, PlayerNetState::Alive))
        .map(|p| (p.position, p.position.distance_squared(enemy_pos)))
        .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(pos, _)| pos)
}

fn nearest_alive_player_id(position: Vec2, players: &Query<&NetPlayer>) -> Option<PlayerId> {
    players
        .iter()
        .filter(|p| matches!(p.state, PlayerNetState::Alive))
        .map(|p| (p.player_id, p.position.distance(position)))
        .min_by(|(pa, a), (pb, b)| {
            a.partial_cmp(b)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(pa.0.cmp(&pb.0))
        })
        .map(|(pid, _)| pid)
}
