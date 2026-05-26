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
use carcinisation_fps_core::spidey::{SpideySim, SpideySimConfig, SpideySimState, tick_spidey_sim};

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

/// Per-enemy Spidey simulation state, attached at spawn time.
///
/// Stores the `SpideySim` fields that persist across ticks.
/// The sim config is shared via `ServerSpideySimConfig`.
#[derive(Component, Debug, Clone)]
pub struct ServerSpideySim {
    pub web_cooldown: f32,
    pub lunge_cooldown: f32,
    pub web_anim_elapsed: Option<f32>,
    pub sim_state: SpideySimState,
    /// Stable per-instance seed for deterministic sim decisions.
    pub seed: u32,
}

impl Default for ServerSpideySim {
    fn default() -> Self {
        Self {
            web_cooldown: 0.0,
            lunge_cooldown: 0.0,
            web_anim_elapsed: None,
            sim_state: SpideySimState::Idle,
            seed: 0,
        }
    }
}

/// Per-enemy Spidey sim config, attached at spawn time.
/// Allows map-authored speed overrides.
#[derive(Component, Debug, Clone)]
pub struct ServerSpideySimConfig(pub SpideySimConfig);

impl ServerSpideySimConfig {
    /// Build from a loaded `FpsCombatConfig` with map-authored speed override.
    #[must_use]
    pub fn from_combat_config(combat: &FpsCombatConfig, speed: f32) -> Self {
        Self(combat.spidey_sim_config().with_authored_speed(speed))
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
    pub projectile_type: NetProjectileType,
}

/// System set for enemy attack spawning.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct EnemyAttackSet;

/// Global counter for projectile `NetworkObjectId`s.
#[derive(Resource, Default)]
pub struct NextProjectileId(pub u32);

impl NextProjectileId {
    pub const fn allocate(&mut self) -> NetworkObjectId {
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
                    projectile_type: NetProjectileType::BloodShot,
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

    apply_melee_hits(&melee_hits, &mut commands, &mut player_health);
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
                    projectile_type: p.projectile_type,
                },
                super::projectile::ProjectileTtl(combat_config.projectile_lifetime),
                Replicated,
            ));
            commands.entity(entity).despawn();
        }
    }
}

/// Map `MosquitonSimState` → `NetEnemyState` for replication.
///
/// Attack type (melee vs ranged) is conveyed via `EnemyAttackVisual` events,
/// not encoded here. See [`NetEnemyState`] doc for the late-joiner trade-off.
const fn sim_state_to_net(state: &MosquitonSimState) -> NetEnemyState {
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

/// Apply deferred melee hits to player health and broadcast damage effects.
fn apply_melee_hits(
    melee_hits: &[MeleeHit],
    commands: &mut Commands,
    player_health: &mut Query<(&NetPlayer, &mut NetHealth), Without<NetEnemy>>,
) {
    for hit in melee_hits {
        for (player, mut health) in &mut *player_health {
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

/// Map `SpideySimState` → `NetEnemyState` for replication.
///
/// Attack type (web vs lunge) is intentionally not encoded here — one-shot
/// attack animations are driven by `EnemyAttackVisual` events per the net
/// protocol design. Late-joining clients will see `HoldingRange` (→ Recover
/// presentation) for mid-attack enemies until the next event fires.
const fn spidey_sim_state_to_net(state: &SpideySimState) -> NetEnemyState {
    match state {
        SpideySimState::Idle => NetEnemyState::Idle,
        SpideySimState::HopWait { .. } | SpideySimState::HopMove { .. } => NetEnemyState::Chase,
        SpideySimState::WebWindup { .. }
        | SpideySimState::LungeWindup { .. }
        | SpideySimState::LungeAttack { .. }
        | SpideySimState::Recover { .. } => NetEnemyState::HoldingRange,
        SpideySimState::Dying { .. } => NetEnemyState::Dying { burn: false },
        SpideySimState::BurningCorpse { .. } => NetEnemyState::Dying { burn: true },
        SpideySimState::Dead => NetEnemyState::Dead { burn: false },
    }
}

/// Tick Spidey enemies using the shared `fps_core` simulation.
///
/// Handles hop movement, web/leap attacks, and melee damage in one call
/// via `tick_spidey_sim`. Maps outputs to ECS/network effects.
#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::cast_precision_loss
)]
pub fn tick_spidey_attacks(
    mut commands: Commands,
    mut enemies: Query<(
        Entity,
        &mut NetEnemy,
        &NetHealth,
        &mut ServerSpideySim,
        &ServerSpideySimConfig,
    )>,
    players: Query<&NetPlayer>,
    mut player_health: Query<(&NetPlayer, &mut NetHealth), Without<NetEnemy>>,
    fixed_time: Res<Time<Fixed>>,
    mut next_id: ResMut<NextProjectileId>,
    server_map: Res<ServerMap>,
) {
    let dt = fixed_time.delta_secs();
    let mut melee_hits: Vec<MeleeHit> = Vec::new();

    for (_enemy_entity, mut enemy, health, mut spidey_sim, sim_config) in &mut enemies {
        if enemy.enemy_type != NetEnemyType::Spidey {
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
        let mut sim = SpideySim {
            position: enemy.position,
            state: spidey_sim.sim_state.clone(),
            web_cooldown: spidey_sim.web_cooldown,
            lunge_cooldown: spidey_sim.lunge_cooldown,
            web_anim_elapsed: spidey_sim.web_anim_elapsed,
            seed: spidey_sim.seed,
        };

        let output = tick_spidey_sim(&mut sim, &sim_config.0, player_pos, &server_map.0, dt);

        // Write back sim state.
        enemy.position = sim.position;
        enemy.visual_height = output.visual_height;
        // During death states, repurpose visual_phase for death animation progress.
        enemy.visual_phase = match &sim.state {
            SpideySimState::Dying { timer } => {
                1.0 - (timer / sim_config.0.death_secs.max(f32::EPSILON)).clamp(0.0, 1.0)
            }
            SpideySimState::BurningCorpse { timer, .. } => {
                // Use death_secs as the reference duration for consistency.
                1.0 - (timer / sim_config.0.death_secs.max(f32::EPSILON)).clamp(0.0, 1.0)
            }
            _ => output.visual_phase,
        };
        spidey_sim.sim_state = sim.state.clone();
        spidey_sim.web_cooldown = sim.web_cooldown;
        spidey_sim.lunge_cooldown = sim.lunge_cooldown;
        spidey_sim.web_anim_elapsed = sim.web_anim_elapsed;
        spidey_sim.seed = sim.seed;

        // Map sim state -> NetEnemyState for replication.
        let net_state = spidey_sim_state_to_net(&sim.state);
        if !matches!(
            enemy.state,
            NetEnemyState::Dying { .. } | NetEnemyState::Dead { .. }
        ) {
            enemy.state = net_state;
        }

        // Handle web animation start -> visual event.
        if output.started_web_anim {
            commands.server_trigger(ToClients {
                mode: SendMode::Broadcast,
                message: EnemyAttackVisual {
                    object_id: enemy.object_id,
                    kind: EnemyAttackKind::Ranged,
                },
            });
        }

        // Consume sim projectile directly (fires at cue time with current aim).
        // Unlike Mosquiton's PendingProjectile approach, this avoids stale aim:
        // the core sim recalculates direction at fire time, not wind-up start.
        if let Some(proj) = &output.projectile {
            let object_id = next_id.allocate();
            let angle = proj.direction.y.atan2(proj.direction.x);
            commands.spawn((
                NetProjectile {
                    object_id,
                    position: proj.position,
                    angle,
                    owner: Owner(PlayerId(0)),
                    damage: proj.damage as f32,
                    projectile_type: NetProjectileType::WebShot,
                },
                super::projectile::ProjectileTtl(proj.lifetime),
                Replicated,
            ));
            debug!(
                "Spidey {:?} fired web projectile {:?} angle={:.2}",
                enemy.object_id, object_id, angle
            );
        }

        // Handle leap start -> visual event.
        if output.started_lunge {
            commands.server_trigger(ToClients {
                mode: SendMode::Broadcast,
                message: EnemyAttackVisual {
                    object_id: enemy.object_id,
                    kind: EnemyAttackKind::Melee,
                },
            });
        }

        // Handle melee damage output (leap arrival).
        if let Some((damage, _source)) = output.melee_damage
            && let Some(target_pid) = nearest_alive_player_id(enemy.position, &players)
        {
            melee_hits.push(MeleeHit {
                target_player_id: target_pid,
                damage: damage as f32,
            });
            debug!(
                "Spidey {:?} leap hit player {:?}",
                enemy.object_id, target_pid
            );
        }
    }

    apply_melee_hits(&melee_hits, &mut commands, &mut player_health);
}
