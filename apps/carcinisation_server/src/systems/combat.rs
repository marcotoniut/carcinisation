//! Server-authoritative combat system.
//!
//! Runs in `FixedUpdate` (`CombatSet`) after movement.
//! - Pistol (`NetAttackId::None`): hitscan + cooldown
//! - Flamethrower (`NetAttackId::Projectile`): continuous cone damage while held

use crate::ServerMap;
use bevy::prelude::*;
use bevy_replicon::prelude::ServerTriggerExt;
use bevy_replicon::prelude::*;
use carcinisation_fps_core::camera::Camera;
use carcinisation_fps_core::config::{
    BURN_CONTACT_DAMAGE, BURN_CONTACT_RADIUS, BURN_CONTACT_TICK_SECS, ENEMY_DEATH_ANIM_SECS,
    ENEMY_DESPAWN_DELAY, FIRE_COOLDOWN_SECS, FLAME_DPS, FLAME_HALF_ANGLE, FLAME_RANGE,
    HITSCAN_DAMAGE, PROJECTILE_HIT_RADIUS,
};
use carcinisation_fps_core::enemy::{Enemy, hitscan};
use carcinisation_fps_core::raycast::cast_ray;
use carcinisation_net::{
    DamageEffect, DeathEffect, FlameActive, FlameCharMark, HitConfirm, MuzzleFlash, NetAttackId,
    NetPlayer, NetProjectile, NetworkObjectId, PlayerId,
};
use std::collections::HashMap;

use crate::systems::NetEnemy;
use crate::systems::NetEnemyState;
use crate::systems::NetHealth;

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
            .any(|pos| pos.distance(player.position) <= BURN_CONTACT_RADIUS);
        if !near {
            continue;
        }
        *cd = BURN_CONTACT_TICK_SECS;
        health.current = (health.current - BURN_CONTACT_DAMAGE).max(0.0);
        commands.server_trigger(ToClients {
            mode: SendMode::Broadcast,
            message: DamageEffect {
                target_id: NetworkObjectId(player.player_id.0),
                damage: BURN_CONTACT_DAMAGE,
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
    clippy::cast_sign_loss
)]
pub fn process_combat(
    mut commands: Commands,
    buffer: Res<super::PlayerIntentBuffer>,
    players: Query<&NetPlayer>,
    mut enemies: Query<(Entity, &mut NetEnemy, &mut NetHealth)>,
    projectiles: Query<(Entity, &NetProjectile)>,
    server_map: Res<ServerMap>,
    fixed_time: Res<Time<Fixed>>,
    mut cooldowns: ResMut<FireCooldownMap>,
    mut flame_tracker: ResMut<FlameActiveTracker>,
    mut char_cooldowns: ResMut<FlameCharCooldowns>,
) {
    let dt = fixed_time.delta_secs();

    // Tick pistol cooldowns.
    for cd in cooldowns.0.values_mut() {
        *cd = (*cd - dt).max(0.0);
    }

    // Weapon switch and snap turns are now handled in apply_buffered_movement (MovementSet)
    // via take_actions(). Combat only reads fire_held from the intent buffer.

    // Combat processing per player (alive only).
    for player in &players {
        if !matches!(player.state, carcinisation_net::PlayerNetState::Alive) {
            continue;
        }
        let firing = buffer.peek_fire_held(&player.player_id);

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
                send_flame_stop(&mut commands, &mut flame_tracker, player);

                if !firing {
                    continue;
                }

                let cd = cooldowns.0.entry(player.player_id).or_insert(0.0);
                if *cd > 0.0 {
                    continue;
                }
                *cd = FIRE_COOLDOWN_SECS;

                let camera = Camera {
                    position: player.position,
                    angle: player.angle,
                    ..Camera::default()
                };

                commands.server_trigger(ToClients {
                    mode: SendMode::Broadcast,
                    message: MuzzleFlash {
                        player_id: player.player_id,
                        position: player.position,
                        angle: player.angle,
                    },
                });

                // Build enemy list for hitscan.
                let mut enemy_entities: Vec<Entity> = Vec::new();
                let mut enemy_list: Vec<Enemy> = Vec::new();
                for (entity, net_enemy, net_health) in enemies.iter() {
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
                }

                let result = hitscan(&camera, &enemy_list, &server_map.0);

                // Also check hitscan against enemy projectiles (player can shoot them down).
                let wall_dist = result.distance;
                let mut closest_proj: Option<(Entity, NetworkObjectId, Vec2, f32)> = None;
                for (proj_entity, proj) in projectiles.iter() {
                    let to_proj = proj.position - camera.position;
                    let along = to_proj.dot(camera.direction());
                    if along <= 0.0 || along > wall_dist {
                        continue;
                    }
                    let perp_sq = to_proj.length_squared() - along * along;
                    if perp_sq < PROJECTILE_HIT_RADIUS * PROJECTILE_HIT_RADIUS
                        && closest_proj.is_none_or(|(_, _, _, d)| along < d)
                    {
                        closest_proj = Some((proj_entity, proj.object_id, proj.position, along));
                    }
                }

                // Determine what was hit first: enemy or projectile.
                let enemy_hit = result.enemy_idx.map(|idx| (idx, result.distance));
                let proj_hit = closest_proj;

                // Projectile closer than enemy?
                if let Some((proj_e, proj_id, proj_pos, proj_d)) = proj_hit
                    && enemy_hit.is_none_or(|(_, ed)| proj_d < ed)
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
                        },
                    });
                    continue;
                }

                let Some((hit_idx, _)) = enemy_hit else {
                    continue;
                };

                let hit_entity = enemy_entities[hit_idx];
                apply_damage(
                    &mut commands,
                    &mut enemies,
                    hit_entity,
                    HITSCAN_DAMAGE,
                    false,
                );

                // Send hit confirmation for blood splat visual.
                if let Ok((_, hit_enemy, _)) = enemies.get(hit_entity) {
                    commands.server_trigger(ToClients {
                        mode: SendMode::Broadcast,
                        message: HitConfirm {
                            target_id: hit_enemy.object_id,
                            damage: HITSCAN_DAMAGE,
                            position: hit_enemy.position,
                            kind: carcinisation_net::HitImpactKind::Hit,
                        },
                    });
                }
            }

            NetAttackId::Projectile => {
                // -- Flamethrower: continuous cone damage --
                if !firing {
                    send_flame_stop(&mut commands, &mut flame_tracker, player);
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
                    commands.server_trigger(ToClients {
                        mode: SendMode::Broadcast,
                        message: FlameActive {
                            player_id: player.player_id,
                            active: true,
                        },
                    });
                }

                let dir = Vec2::new(player.angle.cos(), player.angle.sin());
                let cos_threshold = FLAME_HALF_ANGLE.cos();
                let damage_this_tick = FLAME_DPS * dt;

                debug!(
                    "Flame tick: player={:?} pos={} angle={:.2} dir={} range={} dmg_tick={:.1}",
                    player.player_id,
                    player.position,
                    player.angle,
                    dir,
                    FLAME_RANGE,
                    damage_this_tick
                );

                // Collect hit entities first (avoids borrow conflict with apply_damage).
                let hit_entities: Vec<Entity> = enemies
                    .iter()
                    .filter_map(|(entity, net_enemy, net_health)| {
                        if matches!(
                            net_enemy.state,
                            NetEnemyState::Dying { .. } | NetEnemyState::Dead { .. }
                        ) {
                            return None;
                        }
                        let to_enemy = net_enemy.position - player.position;
                        let dist = to_enemy.length();
                        let in_range = (0.01..=FLAME_RANGE).contains(&dist);
                        let to_dir = if dist > 0.001 { to_enemy / dist } else { Vec2::ZERO };
                        let dot = dir.dot(to_dir);
                        let in_angle = dot >= cos_threshold;
                        let ray_hit = cast_ray(&server_map.0, player.position, to_dir);
                        let has_los = ray_hit.distance >= dist;

                        debug!(
                            "  enemy obj={:?} pos={} hp={:.0} dist={:.2} in_range={} dot={:.3} (thresh={:.3}) in_angle={} los_dist={:.2} has_los={}",
                            net_enemy.object_id, net_enemy.position, net_health.current,
                            dist, in_range, dot, cos_threshold, in_angle, ray_hit.distance, has_los
                        );

                        if !in_range || !in_angle || !has_los {
                            return None;
                        }
                        Some(entity)
                    })
                    .collect();

                for entity in &hit_entities {
                    apply_damage(&mut commands, &mut enemies, *entity, damage_this_tick, true);
                }
                if !hit_entities.is_empty() {
                    debug!(
                        "  Flame hit {} enemies, {:.1} dmg each",
                        hit_entities.len(),
                        damage_this_tick
                    );
                }

                // Wall char mark emission (rate-limited).
                let char_cd = char_cooldowns.0.entry(player.player_id).or_insert(0.0);
                *char_cd = (*char_cd - dt).max(0.0);
                if *char_cd <= 0.0 {
                    let wall_hit = cast_ray(&server_map.0, player.position, dir);
                    if wall_hit.wall_id > 0 && wall_hit.distance <= FLAME_RANGE {
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
}

/// Send FlameActive(false) if the player was previously flaming.
fn send_flame_stop(commands: &mut Commands, tracker: &mut FlameActiveTracker, player: &NetPlayer) {
    if tracker.0.get(&player.player_id).copied().unwrap_or(false) {
        tracker.0.insert(player.player_id, false);
        commands.server_trigger(ToClients {
            mode: SendMode::Broadcast,
            message: FlameActive {
                player_id: player.player_id,
                active: false,
            },
        });
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

/// Apply damage to an enemy entity, handling death transition and events.
fn apply_damage(
    commands: &mut Commands,
    enemies: &mut Query<(Entity, &mut NetEnemy, &mut NetHealth)>,
    entity: Entity,
    damage: f32,
    burn: bool,
) {
    let Ok((_, mut net_enemy, mut net_health)) = enemies.get_mut(entity) else {
        return;
    };
    if net_health.current <= 0.0 {
        return;
    }

    net_health.current = (net_health.current - damage).max(0.0);

    if net_health.current <= 0.0 {
        net_enemy.state = NetEnemyState::Dying { burn };
        commands
            .entity(entity)
            .insert(EnemyDeathTimer(ENEMY_DEATH_ANIM_SECS))
            .insert(DespawnTimer(ENEMY_DESPAWN_DELAY));
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
