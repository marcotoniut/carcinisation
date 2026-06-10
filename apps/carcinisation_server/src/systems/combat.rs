//! Server-authoritative combat system.
//!
//! Runs in `FixedUpdate` (`CombatSet`) after movement.
//! - Pistol (`NetAttackId::None`): hitscan + cooldown
//! - Flamethrower (`NetAttackId::Projectile`): progressive burn via `BurnState`

use crate::ServerMap;
use bevy::prelude::*;
use bevy_replicon::prelude::ServerTriggerExt;
use bevy_replicon::prelude::*;
use carcinisation_fps_core::burning::{self, BurnConfig, BurnState};
use carcinisation_fps_core::collision_set;
use carcinisation_fps_core::combat::{FirePose2d, wall_obstruction_distance_for_pose};
use carcinisation_fps_core::config::FpsCombatConfig;
use carcinisation_fps_core::enemy::{Enemy, FpsEnemyKind};
use carcinisation_fps_core::enemy_collision::{
    DEFAULT_ANIMATION, DEFAULT_FRAME, enemy_fallback_radius,
};
use carcinisation_fps_core::fire_death::corpse_seed;
use carcinisation_fps_core::hitscan::{
    PartHitscanTarget, flame_hits_target_parts_configured, hitscan_parts_from_pose, scaled_damage,
};
use carcinisation_fps_core::raycast::cast_ray;
use carcinisation_net::{
    DamageEffect, DeathEffect, FlameActive, FlameCharMark, HitConfirm, MuzzleFlash, NetAttackId,
    NetBurning, NetGroundFire, NetPlayer, NetProjectile, NetProjectileType, NetworkObjectId,
    PlayerId,
};
use std::collections::HashMap;

use crate::systems::NetEnemy;
use crate::systems::NetHealth;
use crate::systems::{NetEnemyState, NetEnemyType};

/// Map the replicated enemy type to the shared collision fixture kind.
const fn fps_kind_from_net(enemy_type: NetEnemyType) -> FpsEnemyKind {
    match enemy_type {
        NetEnemyType::Basic => FpsEnemyKind::Basic,
        NetEnemyType::Mosquiton => FpsEnemyKind::Mosquiton,
        NetEnemyType::Spidey => FpsEnemyKind::Spidey,
    }
}

/// Server-only burn state wrapper. Not replicated — intensity is synced to `NetBurning`.
#[derive(Component, Debug, Clone, Default)]
pub struct ServerBurnState(pub BurnState);

/// Load `BurnConfig` from the embedded RON asset.
#[must_use]
pub fn load_burn_config() -> BurnConfig {
    burning::load_config()
}

/// Per-player cooldown for burning corpse contact damage.
#[derive(Resource, Default)]
pub struct BurnContactCooldowns(pub HashMap<PlayerId, f32>);

impl BurnContactCooldowns {
    pub fn remove_player(&mut self, player_id: &PlayerId) {
        self.0.remove(player_id);
    }
}

/// Server-authoritative burning corpse proximity damage.
pub fn tick_burn_contact_damage(
    mut commands: Commands,
    mut players: Query<(&NetPlayer, &mut NetHealth)>,
    enemies: Query<&NetEnemy>,
    mut cooldowns: ResMut<BurnContactCooldowns>,
    fixed_time: Res<Time<Fixed>>,
    combat_config: Res<FpsCombatConfig>,
) {
    let dt = fixed_time.delta_secs();

    // Tick cooldowns.
    for cd in cooldowns.0.values_mut() {
        *cd = (*cd - dt).max(0.0);
    }

    // Collect burning corpse positions.
    let burning: Vec<Vec2> = enemies
        .iter()
        .filter(|e| {
            matches!(
                e.state,
                NetEnemyState::Dying { burn: true } | NetEnemyState::Dead { burn: true }
            )
        })
        .map(|e| e.position)
        .collect();

    if burning.is_empty() {
        return;
    }

    for (player, mut health) in &mut players {
        if !matches!(player.state, carcinisation_net::PlayerNetState::Alive)
            || health.current <= 0.0
        {
            continue;
        }
        let cd = cooldowns.0.entry(player.player_id).or_insert(0.0);
        if *cd > 0.0 {
            continue;
        }
        // Check proximity to any burning corpse.
        let near = burning
            .iter()
            .any(|pos| pos.distance(player.position) <= combat_config.burn_contact_radius);
        if !near {
            continue;
        }
        *cd = combat_config.burn_contact_tick_secs;
        health.current = (health.current - combat_config.burn_contact_damage).max(0.0);
        commands.server_trigger(ToClients {
            mode: SendMode::Broadcast,
            message: DamageEffect {
                target_id: NetworkObjectId(player.player_id.0),
                damage: combat_config.burn_contact_damage,
                remaining_health: health.current,
                was_player: true,
            },
        });
    }
}

/// Server-only timer to despawn dead enemies after a delay.
#[derive(Component, Debug, Clone, Copy)]
pub struct DespawnTimer(pub f32);

/// Tick despawn timers and remove entities when expired.
pub fn tick_despawn_timers(
    mut commands: Commands,
    mut timers: Query<(Entity, &mut DespawnTimer)>,
    fixed_time: Res<Time<Fixed>>,
) {
    let dt = fixed_time.delta_secs();
    for (entity, mut timer) in &mut timers {
        timer.0 -= dt;
        if timer.0 <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

/// Per-player fire cooldown tracker (pistol only).
#[derive(Resource, Default)]
pub struct FireCooldownMap(pub HashMap<PlayerId, f32>);

impl FireCooldownMap {
    pub fn remove_player(&mut self, player_id: &PlayerId) {
        self.0.remove(player_id);
    }
}

/// Tracks which players are currently flaming (for `FlameActive` start/stop events).
#[derive(Resource, Default)]
pub struct FlameActiveTracker(pub HashMap<PlayerId, bool>);

impl FlameActiveTracker {
    pub fn remove_player(&mut self, player_id: &PlayerId) {
        self.0.remove(player_id);
    }
}

/// Rate-limiter for `FlameCharMark` events (one per player per ~100ms).
#[derive(Resource, Default)]
pub struct FlameCharCooldowns(pub HashMap<PlayerId, f32>);

impl FlameCharCooldowns {
    pub fn remove_player(&mut self, player_id: &PlayerId) {
        self.0.remove(player_id);
    }
}

/// Interval between wall char mark events per player (seconds).
const FLAME_CHAR_EMIT_INTERVAL: f32 = 0.1;

#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::missing_panics_doc
)]
pub fn process_combat(
    mut commands: Commands,
    buffer: Res<super::PlayerIntentBuffer>,
    mut players: Query<&mut NetPlayer>,
    mut enemies: Query<
        (Entity, &mut NetEnemy, &mut NetHealth, &mut ServerBurnState),
        Without<NetPlayer>,
    >,
    projectiles: Query<(Entity, &NetProjectile)>,
    server_map: Res<ServerMap>,
    fixed_time: Res<Time<Fixed>>,
    mut cooldowns: ResMut<FireCooldownMap>,
    mut flame_tracker: ResMut<FlameActiveTracker>,
    mut char_cooldowns: ResMut<FlameCharCooldowns>,
    burn_config: Res<BurnConfig>,
    flame_cfg: Res<carcinisation_fps_core::PlayerFlamethrowerConfig>,
    combat_config: Res<FpsCombatConfig>,
) {
    let dt = fixed_time.delta_secs();

    // Tick pistol cooldowns.
    for cd in cooldowns.0.values_mut() {
        *cd = (*cd - dt).max(0.0);
    }

    // Enemy snapshot Vecs — declared once so `clear()` + `push()` reuses
    // capacity instead of allocating per player.
    //
    // IMPORTANT: the rebuild MUST stay inside the per-player hitscan branch.
    // `apply_damage` mutates `NetHealth`/`NetEnemyState` on the live query,
    // so an earlier player's kill must be visible to later players in the
    // same tick. A shared pre-loop snapshot would let later players waste
    // shots on already-dead enemies.
    let mut enemy_entities: Vec<Entity> = Vec::new();
    let mut enemy_list: Vec<Enemy> = Vec::new();
    // Parallel to `enemy_list`: authoritative collision identity (kind, yaw)
    // read from the replicated `NetEnemy` (enemy_type / angle).
    let mut enemy_meta: Vec<(FpsEnemyKind, f32)> = Vec::new();

    // Entities hit by any flamethrower this tick — exposure applied after the player loop.
    let mut flame_exposed_entities: Vec<Entity> = Vec::new();

    // Flame state changes to apply after combat (avoids mutable borrow during iteration).
    let mut flame_updates: HashMap<PlayerId, bool> = HashMap::new();

    let aim_mode = matches!(
        combat_config.combat_control_mode,
        carcinisation_fps_core::CombatControlMode::AimCommitment
    );

    // Combat processing per player (alive only).
    for player in &players {
        if !matches!(player.state, carcinisation_net::PlayerNetState::Alive) {
            continue;
        }
        let raw_firing = buffer.peek_fire_held(&player.player_id);
        let aim_held = buffer.peek_aim_held(&player.player_id);
        // AimCommitment: fire only while aiming. Legacy: fire anytime.
        let firing = if aim_mode {
            raw_firing && aim_held
        } else {
            raw_firing
        };
        let fire_pose = FirePose2d::new(player.position, player.angle, 0.0);

        if firing {
            debug!(
                "Player {:?} combat: attack={:?} fire_held=true",
                player.player_id, player.current_attack
            );
        }

        match player.current_attack {
            NetAttackId::None | NetAttackId::Melee => {
                // -- Pistol: hitscan + cooldown --
                // Stop flame if was active.
                if send_flame_stop(&mut commands, &mut flame_tracker, player) {
                    flame_updates.insert(player.player_id, false);
                }

                if !firing {
                    continue;
                }

                let cd = cooldowns.0.entry(player.player_id).or_insert(0.0);
                if *cd > 0.0 {
                    continue;
                }
                *cd = combat_config.fire_cooldown_secs;

                commands.server_trigger(ToClients {
                    mode: SendMode::Broadcast,
                    message: MuzzleFlash {
                        player_id: player.player_id,
                        position: fire_pose.origin_xy,
                        angle: fire_pose.yaw,
                    },
                });

                // Rebuild enemy list with fresh health/state (Vecs reuse capacity).
                enemy_entities.clear();
                enemy_list.clear();
                enemy_meta.clear();
                for (entity, net_enemy, net_health, _) in enemies.iter() {
                    if matches!(
                        net_enemy.state,
                        NetEnemyState::Dying { .. } | NetEnemyState::Dead { .. }
                    ) || net_health.current <= 0.0
                    {
                        continue;
                    }
                    enemy_entities.push(entity);
                    enemy_list.push(Enemy::new(
                        net_enemy.position,
                        net_health.current as u32,
                        0.0,
                    ));
                    enemy_meta.push((fps_kind_from_net(net_enemy.enemy_type), net_enemy.angle));
                }

                // Per-part hitscan using authoritative collision identity:
                // per-enemy kind (NetEnemy.enemy_type) selects the fixture and
                // per-enemy yaw (NetEnemy.angle) selects the billboard facing.
                // Frame is DEFAULT_FRAME=0: no discrete enemy animation frame is
                // replicated yet (see Phase 3.5 limitations / TODO).
                let part_hit = hitscan_parts_from_pose(
                    fire_pose,
                    &server_map.0,
                    enemy_list
                        .iter()
                        .zip(enemy_meta.iter())
                        .map(|(e, &(kind, yaw))| PartHitscanTarget {
                            position: e.position,
                            yaw,
                            alive: e.is_alive(),
                            set: collision_set(kind),
                            animation: DEFAULT_ANIMATION,
                            frame: DEFAULT_FRAME,
                            fallback_radius: enemy_fallback_radius(kind, &combat_config),
                        }),
                );

                // Also check hitscan against enemy projectiles (player can shoot them down).
                let wall_dist =
                    wall_obstruction_distance_for_pose(&server_map.0, fire_pose, f32::MAX);
                let mut closest_proj: Option<(
                    Entity,
                    NetworkObjectId,
                    Vec2,
                    f32,
                    NetProjectileType,
                )> = None;
                for (proj_entity, proj) in projectiles.iter() {
                    let to_proj = proj.position - fire_pose.origin_xy;
                    let along = to_proj.dot(fire_pose.direction());
                    if along <= 0.0 || along > wall_dist {
                        continue;
                    }
                    let perp_sq = along.mul_add(-along, to_proj.length_squared());
                    if perp_sq
                        < combat_config.projectile_hit_radius * combat_config.projectile_hit_radius
                        && closest_proj.is_none_or(|(_, _, _, d, _)| along < d)
                    {
                        closest_proj = Some((
                            proj_entity,
                            proj.object_id,
                            proj.position,
                            along,
                            proj.projectile_type,
                        ));
                    }
                }

                // Determine what was hit first: enemy or projectile.
                let enemy_hit = part_hit.map(|r| (r.target_idx, r.distance, r.damage_scale));
                let proj_hit = closest_proj;

                // Projectile closer than enemy?
                if let Some((proj_e, proj_id, proj_pos, proj_d, proj_type)) = proj_hit
                    && enemy_hit.is_none_or(|(_, ed, _)| proj_d < ed)
                {
                    // Destroy the projectile.
                    commands.entity(proj_e).despawn();
                    commands.server_trigger(ToClients {
                        mode: SendMode::Broadcast,
                        message: HitConfirm {
                            target_id: proj_id,
                            damage: 0.0,
                            position: proj_pos,
                            kind: carcinisation_net::HitImpactKind::Destroy,
                            projectile_type: Some(proj_type),
                        },
                    });
                    continue;
                }

                let Some((hit_idx, _, damage_scale)) = enemy_hit else {
                    continue;
                };

                // Server-authoritative part damage routing: base hitscan damage
                // scaled by the hit part's multiplier (e.g. 2× headshot). The
                // client never scales — it only renders this `HitConfirm`.
                let dealt = scaled_damage(combat_config.hitscan_damage, damage_scale);

                let hit_entity = enemy_entities[hit_idx];
                apply_damage(
                    &mut commands,
                    &mut enemies,
                    hit_entity,
                    dealt,
                    false,
                    &combat_config,
                );

                // Send hit confirmation for blood splat visual. Use the part
                // surface hit point so the effect aligns with where the shot
                // landed rather than the enemy centre.
                if let Ok((_, hit_enemy, _, _)) = enemies.get(hit_entity) {
                    let impact_pos = part_hit.map_or(hit_enemy.position, |r| r.point);
                    commands.server_trigger(ToClients {
                        mode: SendMode::Broadcast,
                        message: HitConfirm {
                            target_id: hit_enemy.object_id,
                            damage: dealt,
                            position: impact_pos,
                            kind: carcinisation_net::HitImpactKind::Hit,
                            projectile_type: None,
                        },
                    });
                }
            }

            NetAttackId::Projectile => {
                // -- Flamethrower: continuous cone damage --
                if !firing {
                    if send_flame_stop(&mut commands, &mut flame_tracker, player) {
                        flame_updates.insert(player.player_id, false);
                    }
                    continue;
                }

                // Send FlameActive start if not already active.
                let was_active = flame_tracker
                    .0
                    .get(&player.player_id)
                    .copied()
                    .unwrap_or(false);
                if !was_active {
                    flame_tracker.0.insert(player.player_id, true);
                    flame_updates.insert(player.player_id, true);
                    commands.server_trigger(ToClients {
                        mode: SendMode::Broadcast,
                        message: FlameActive {
                            player_id: player.player_id,
                            active: true,
                        },
                    });
                }

                let dir = fire_pose.direction();

                // Collect hit entities and apply burn exposure (replaces instant
                // damage). Per-part flame overlap using authoritative kind/yaw,
                // matching the hitscan target setup. Frame is DEFAULT_FRAME=0.
                for (entity, net_enemy, _, _) in enemies.iter() {
                    if matches!(
                        net_enemy.state,
                        NetEnemyState::Dying { .. } | NetEnemyState::Dead { .. }
                    ) {
                        continue;
                    }
                    let kind = fps_kind_from_net(net_enemy.enemy_type);
                    let target = PartHitscanTarget {
                        position: net_enemy.position,
                        yaw: net_enemy.angle,
                        alive: true,
                        set: collision_set(kind),
                        animation: DEFAULT_ANIMATION,
                        frame: DEFAULT_FRAME,
                        fallback_radius: enemy_fallback_radius(kind, &combat_config),
                    };
                    if flame_hits_target_parts_configured(
                        fire_pose,
                        &server_map.0,
                        &flame_cfg,
                        target,
                    )
                    .is_some()
                    {
                        flame_exposed_entities.push(entity);
                    }
                }

                // Wall char mark emission (rate-limited).
                let char_cd = char_cooldowns.0.entry(player.player_id).or_insert(0.0);
                *char_cd = (*char_cd - dt).max(0.0);
                if *char_cd <= 0.0 {
                    let wall_hit = cast_ray(&server_map.0, fire_pose.origin_xy, dir);
                    if wall_hit.wall_id > 0
                        && wall_hit.distance
                            <= wall_obstruction_distance_for_pose(
                                &server_map.0,
                                fire_pose,
                                flame_cfg.range,
                            )
                    {
                        *char_cd = FLAME_CHAR_EMIT_INTERVAL;
                        let side = match wall_hit.side {
                            carcinisation_fps_core::HitSide::Vertical => 0u8,
                            carcinisation_fps_core::HitSide::Horizontal => 1u8,
                        };
                        let surface = wall_hit.surface_id.unwrap();
                        // FNV-1a seed from cell + UV.
                        let u_quant = (wall_hit.wall_x * 4096.0) as u32;
                        let seed =
                            2_166_136_261u32.wrapping_mul(16_777_619) ^ surface.cell_x as u32;
                        let seed = seed.wrapping_mul(16_777_619) ^ surface.cell_y as u32;
                        let seed = seed.wrapping_mul(16_777_619) ^ u_quant;
                        let seed = seed.wrapping_mul(16_777_619) ^ surface.normal_sign as u32;
                        commands.server_trigger(ToClients {
                            mode: SendMode::Broadcast,
                            message: FlameCharMark {
                                cell_x: surface.cell_x,
                                cell_y: surface.cell_y,
                                side,
                                normal_sign: surface.normal_sign,
                                u: wall_hit.wall_x,
                                seed,
                            },
                        });
                    }
                }
            }
        }
    }

    // Apply flame_active to replicated NetPlayer components.
    // Collected during the loop to avoid mutable borrow conflicts.
    // bevy_replicon change detection ensures this only replicates on transition.
    if !flame_updates.is_empty() {
        for mut p in &mut players {
            if let Some(&active) = flame_updates.get(&p.player_id) {
                p.flame_active = active;
            }
        }
    }

    // Apply burn exposure to all flame-hit entities (deduplicated).
    flame_exposed_entities.sort_unstable();
    flame_exposed_entities.dedup();
    for entity in &flame_exposed_entities {
        if let Ok((_, _, _, mut burn)) = enemies.get_mut(*entity) {
            burning::apply_exposure(
                &mut burn.0,
                &burn_config,
                burn_config.flame_exposure_per_sec,
                dt,
            );
        }
    }
}

/// Send FlameActive(false) if the player was previously flaming.
/// Returns `true` if a stop transition occurred.
fn send_flame_stop(
    commands: &mut Commands,
    tracker: &mut FlameActiveTracker,
    player: &NetPlayer,
) -> bool {
    if tracker.0.get(&player.player_id).copied().unwrap_or(false) {
        tracker.0.insert(player.player_id, false);
        commands.server_trigger(ToClients {
            mode: SendMode::Broadcast,
            message: FlameActive {
                player_id: player.player_id,
                active: false,
            },
        });
        true
    } else {
        false
    }
}

/// Server-only death animation timer. Transitions enemy from Dying → Dead.
#[derive(Component, Debug, Clone, Copy)]
pub struct EnemyDeathTimer(pub f32);

/// Tick enemy death timers: Dying → Dead after animation period.
pub fn tick_enemy_death_timers(
    mut enemies: Query<(&mut NetEnemy, &mut EnemyDeathTimer)>,
    fixed_time: Res<Time<Fixed>>,
) {
    let dt = fixed_time.delta_secs();
    for (mut enemy, mut timer) in &mut enemies {
        let NetEnemyState::Dying { burn } = enemy.state else {
            continue;
        };
        timer.0 -= dt;
        if timer.0 <= 0.0 {
            enemy.state = NetEnemyState::Dead { burn };
        }
    }
}

/// Server-side ground fire entity count for cap enforcement.
#[derive(Resource, Default)]
pub struct GroundFireCount(pub u32);

/// Per-player cooldown for ground fire contact damage.
#[derive(Resource, Default)]
pub struct GroundFireContactCooldowns(pub HashMap<PlayerId, f32>);

impl GroundFireContactCooldowns {
    pub fn remove_player(&mut self, player_id: &PlayerId) {
        self.0.remove(player_id);
    }
}

/// Server-authoritative ground fire proximity damage.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
pub fn tick_ground_fire_damage(
    mut commands: Commands,
    mut players: Query<(&NetPlayer, &mut NetHealth)>,
    mut enemies: Query<
        (Entity, &mut NetEnemy, &mut NetHealth, &mut ServerBurnState),
        (Without<NetPlayer>, Without<DespawnTimer>),
    >,
    ground_fires: Query<(&NetGroundFire, &DespawnTimer)>,
    mut cooldowns: ResMut<GroundFireContactCooldowns>,
    fixed_time: Res<Time<Fixed>>,
    burn_config: Res<BurnConfig>,
    combat_config: Res<FpsCombatConfig>,
) {
    let dt = fixed_time.delta_secs();

    for cd in cooldowns.0.values_mut() {
        *cd = (*cd - dt).max(0.0);
    }

    // Collect fire positions with intensity (half damage after fade threshold).
    let fade_threshold =
        combat_config.ground_fire_lifetime_secs - combat_config.ground_fire_fade_start_secs;
    let fire_data: Vec<(Vec2, f32)> = ground_fires
        .iter()
        .map(|(f, timer)| {
            let intensity = if timer.0 <= fade_threshold { 0.5 } else { 1.0 };
            (f.position, intensity)
        })
        .collect();
    if fire_data.is_empty() {
        return;
    }

    let radius_sq = combat_config.ground_fire_radius * combat_config.ground_fire_radius;

    // Damage players standing on ground fires.
    for (player, mut health) in &mut players {
        if !matches!(player.state, carcinisation_net::PlayerNetState::Alive)
            || health.current <= 0.0
        {
            continue;
        }
        let cd = cooldowns.0.entry(player.player_id).or_insert(0.0);
        if *cd > 0.0 {
            continue;
        }
        // Find the closest fire within radius.
        let closest = fire_data
            .iter()
            .filter(|(pos, _)| pos.distance_squared(player.position) <= radius_sq)
            .min_by(|(_, a_int), (_, b_int)| {
                a_int
                    .partial_cmp(b_int)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        let Some(&(_, intensity)) = closest else {
            continue;
        };
        *cd = combat_config.ground_fire_tick_secs;
        let damage = combat_config.ground_fire_damage * intensity;
        health.current = (health.current - damage).max(0.0);
        commands.server_trigger(ToClients {
            mode: SendMode::Broadcast,
            message: DamageEffect {
                target_id: NetworkObjectId(player.player_id.0),
                damage,
                remaining_health: health.current,
                was_player: true,
            },
        });
    }

    // Apply burn exposure to alive enemies standing on ground fires (crossfire).
    let dt = fixed_time.delta_secs();
    for (_entity, net_enemy, _, mut burn) in &mut enemies {
        if matches!(
            net_enemy.state,
            NetEnemyState::Dying { .. } | NetEnemyState::Dead { .. }
        ) {
            continue;
        }
        let in_fire = fire_data
            .iter()
            .any(|(pos, _)| pos.distance_squared(net_enemy.position) <= radius_sq);
        if in_fire {
            burning::apply_exposure(
                &mut burn.0,
                &burn_config,
                burn_config.ground_fire_exposure_per_sec,
                dt,
            );
        }
    }
}

/// Tick burn state for all enemies: apply intensity-proportional damage, decay,
/// sync intensity to `NetBurning` for replication, clear on death.
#[allow(clippy::type_complexity)]
pub fn tick_enemy_burning(
    mut commands: Commands,
    mut enemies: Query<
        (
            Entity,
            &mut NetEnemy,
            &mut NetHealth,
            &mut ServerBurnState,
            &mut NetBurning,
        ),
        Without<NetPlayer>,
    >,
    burn_config: Res<BurnConfig>,
    fixed_time: Res<Time<Fixed>>,
    combat_config: Res<FpsCombatConfig>,
) {
    let dt = fixed_time.delta_secs();

    for (entity, mut net_enemy, mut net_health, mut burn, mut net_burning) in &mut enemies {
        // Tick burn (applies damage + decay).
        // TODO: pass actual movement state when enemy movement tracking is added.
        let result = burning::tick_burning(&mut burn.0, &burn_config, dt, false);

        // Skip dead enemies (state may have been set to Dying by this same burn).
        if matches!(
            net_enemy.state,
            NetEnemyState::Dying { .. } | NetEnemyState::Dead { .. }
        ) {
            continue;
        }

        // Sync intensity to replicated component (only on change to avoid replication churn).
        let new_intensity = burn.0.intensity;
        if (net_burning.intensity - new_intensity).abs() > f32::EPSILON {
            net_burning.intensity = new_intensity;
        }

        // Apply burn damage.
        if result.damage > 0 && net_health.current > 0.0 {
            #[allow(clippy::cast_precision_loss)]
            let damage = result.damage as f32;
            net_health.current = (net_health.current - damage).max(0.0);

            if net_health.current <= 0.0 {
                net_enemy.state = NetEnemyState::Dying { burn: true };
                let death_position = net_enemy.position;
                commands
                    .entity(entity)
                    .insert(EnemyDeathTimer(combat_config.enemy_death_anim_secs))
                    .insert(DespawnTimer(combat_config.enemy_despawn_delay));

                // Ground fire on burn death.
                commands.spawn((
                    NetGroundFire {
                        position: death_position,
                        seed: corpse_seed(death_position),
                    },
                    DespawnTimer(combat_config.ground_fire_lifetime_secs),
                    Replicated,
                ));

                burning::extinguish(&mut burn.0);
                net_burning.intensity = 0.0;

                commands.server_trigger(ToClients {
                    mode: SendMode::Broadcast,
                    message: DeathEffect {
                        target_id: net_enemy.object_id,
                        was_player: false,
                    },
                });
            } else {
                commands.server_trigger(ToClients {
                    mode: SendMode::Broadcast,
                    message: DamageEffect {
                        target_id: net_enemy.object_id,
                        damage,
                        remaining_health: net_health.current,
                        was_player: false,
                    },
                });
            }
        }
    }
}

/// Apply damage to an enemy entity, handling death transition and events.
///
/// When `burn` is true and the enemy dies, a ground fire entity is spawned at
/// the death position. The caller must pass `ground_fire_count` to enforce
/// the server-side cap.
fn apply_damage(
    commands: &mut Commands,
    enemies: &mut Query<
        (Entity, &mut NetEnemy, &mut NetHealth, &mut ServerBurnState),
        Without<NetPlayer>,
    >,
    entity: Entity,
    damage: f32,
    burn: bool,
    combat_config: &FpsCombatConfig,
) {
    let Ok((_, mut net_enemy, mut net_health, _)) = enemies.get_mut(entity) else {
        return;
    };
    if net_health.current <= 0.0 {
        return;
    }

    net_health.current = (net_health.current - damage).max(0.0);

    if net_health.current <= 0.0 {
        net_enemy.state = NetEnemyState::Dying { burn };
        let death_position = net_enemy.position;
        commands
            .entity(entity)
            .insert(EnemyDeathTimer(combat_config.enemy_death_anim_secs))
            .insert(DespawnTimer(combat_config.enemy_despawn_delay));

        // Spawn ground fire at death position for fire kills.
        if burn {
            commands.spawn((
                NetGroundFire {
                    position: death_position,
                    seed: corpse_seed(death_position),
                },
                DespawnTimer(combat_config.ground_fire_lifetime_secs),
                Replicated,
            ));
        }

        commands.server_trigger(ToClients {
            mode: SendMode::Broadcast,
            message: DeathEffect {
                target_id: net_enemy.object_id,
                was_player: false,
            },
        });
    } else {
        commands.server_trigger(ToClients {
            mode: SendMode::Broadcast,
            message: DamageEffect {
                target_id: net_enemy.object_id,
                damage,
                remaining_health: net_health.current,
                was_player: false,
            },
        });
    }
}

#[cfg(test)]
mod tests {
    use super::fps_kind_from_net;
    use crate::systems::NetEnemyType;
    use carcinisation_fps_core::FpsEnemyKind;

    #[test]
    fn kind_mapping_matches_net_enemy_type() {
        assert_eq!(fps_kind_from_net(NetEnemyType::Basic), FpsEnemyKind::Basic);
        assert_eq!(
            fps_kind_from_net(NetEnemyType::Mosquiton),
            FpsEnemyKind::Mosquiton
        );
        assert_eq!(
            fps_kind_from_net(NetEnemyType::Spidey),
            FpsEnemyKind::Spidey
        );
    }
}
