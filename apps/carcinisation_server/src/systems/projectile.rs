//! Server-authoritative projectile movement, collision, and damage.
//!
//! Runs in `ProjectileSet` after enemy attacks have spawned new projectiles.
//! Uses `cast_ray` for wall collision and point-vs-circle for player hits.
//! Active flamethrowers destroy projectiles that enter their cone.

use crate::ServerMap;
use bevy::prelude::*;
use bevy_replicon::prelude::*;
use carcinisation_fps_core::combat::flame_hits_position;
use carcinisation_fps_core::config::FpsCombatConfig;
use carcinisation_fps_core::raycast::cast_ray;
use carcinisation_fps_core::segment_circle_hit_distance;
use carcinisation_net::{
    DamageEffect, HitConfirm, NetHealth, NetPlayer, NetProjectile, NetProjectileType,
    NetSpeedModifier, NetworkObjectId, PlayerNetState,
};

use super::FlameActiveTracker;

/// System set for projectile ticking.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct ProjectileSet;

/// Server-only TTL component (not replicated, avoids per-tick replication churn).
#[derive(Component, Debug, Clone, Copy)]
pub struct ProjectileTtl(pub f32);

/// Move projectiles, check collisions, apply damage, despawn.
#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap
)]
pub fn tick_projectiles_server(
    mut commands: Commands,
    mut projectiles: Query<(Entity, &mut NetProjectile, &mut ProjectileTtl)>,
    mut players: Query<(Entity, &NetPlayer, &mut NetHealth)>,
    server_map: Res<ServerMap>,
    fixed_time: Res<Time<Fixed>>,
    flame_tracker: Res<FlameActiveTracker>,
    flame_cfg: Res<carcinisation_fps_core::PlayerFlamethrowerConfig>,
    combat_config: Res<FpsCombatConfig>,
) {
    let dt = fixed_time.delta_secs();

    // Collect active flamers (position + direction) for projectile interception.
    let active_flamers: Vec<(Vec2, Vec2)> = players
        .iter()
        .filter_map(|(_, np, _)| {
            if flame_tracker.0.get(&np.player_id).copied().unwrap_or(false) {
                Some((np.position, Vec2::new(np.angle.cos(), np.angle.sin())))
            } else {
                None
            }
        })
        .collect();

    for (proj_entity, mut proj, mut ttl) in &mut projectiles {
        // Tick TTL.
        ttl.0 -= dt;
        if ttl.0 <= 0.0 {
            commands.entity(proj_entity).despawn();
            continue;
        }

        // Flame interception: active flamethrowers destroy projectiles in their cone.
        let flame_destroyed = active_flamers.iter().any(|&(origin, flame_dir)| {
            flame_hits_position(
                origin,
                flame_dir,
                proj.position,
                flame_cfg.range,
                flame_cfg.hit_half_width,
                &server_map.0,
            )
        });
        if flame_destroyed {
            commands.server_trigger(ToClients {
                mode: SendMode::Broadcast,
                message: HitConfirm {
                    target_id: proj.object_id,
                    damage: 0.0,
                    position: proj.position,
                    kind: carcinisation_net::HitImpactKind::Destroy,
                    projectile_type: Some(proj.projectile_type),
                    part_id: None,
                    critical: false,
                },
            });
            commands.entity(proj_entity).despawn();
            continue;
        }

        let dir = Vec2::new(proj.angle.cos(), proj.angle.sin());
        let projectile_speed = match proj.projectile_type {
            NetProjectileType::BloodShot => combat_config.projectile_speed,
            NetProjectileType::WebShot => combat_config.spidey.web_projectile_speed,
        };
        let step = dir * projectile_speed * dt;
        let travel_distance = step.length();
        if travel_distance <= f32::EPSILON {
            continue;
        }
        let previous = proj.position;
        let next = previous + step;

        // Wall collision via raycast.
        let wall_hit = cast_ray(&server_map.0, previous, dir);
        if wall_hit.wall_id > 0 && wall_hit.distance <= travel_distance {
            let impact_pos = previous + dir * wall_hit.distance;
            commands.server_trigger(ToClients {
                mode: SendMode::Broadcast,
                message: HitConfirm {
                    target_id: proj.object_id,
                    damage: 0.0,
                    position: impact_pos,
                    kind: carcinisation_net::HitImpactKind::Hit,
                    projectile_type: Some(proj.projectile_type),
                    part_id: None,
                    critical: false,
                },
            });
            commands.entity(proj_entity).despawn();
            continue;
        }

        // Player collision — check all alive players (skip owner).
        let mut hit_player: Option<(Entity, f32)> = None;
        for (player_entity, net_player, _) in players.iter() {
            if !matches!(net_player.state, PlayerNetState::Alive) {
                continue;
            }
            // Skip the projectile's owner (prevents self-hit for player projectiles).
            if net_player.player_id == proj.owner.0 {
                continue;
            }
            if let Some(dist) = segment_circle_hit_distance(
                previous,
                next,
                net_player.position,
                combat_config.projectile_hit_radius,
            ) && hit_player.is_none_or(|(_, d)| dist < d)
            {
                hit_player = Some((player_entity, dist));
            }
        }

        if let Some((player_entity, _)) = hit_player {
            // Apply damage to player (skip if already dead).
            if let Ok((_, net_player, mut health)) = players.get_mut(player_entity) {
                if health.current <= 0.0 {
                    commands.entity(proj_entity).despawn();
                    continue;
                }
                health.current = (health.current - proj.damage).max(0.0);
                if matches!(proj.projectile_type, NetProjectileType::WebShot) {
                    commands.entity(player_entity).insert(NetSpeedModifier {
                        multiplier: combat_config.spidey.web_slow_multiplier,
                        remaining: combat_config.spidey.web_slow_duration,
                    });
                }
                debug!(
                    "Projectile {:?} hit player {:?}: {:.0} dmg, hp={:.0}",
                    proj.object_id, net_player.player_id, proj.damage, health.current
                );

                commands.server_trigger(ToClients {
                    mode: SendMode::Broadcast,
                    message: DamageEffect {
                        target_id: NetworkObjectId(net_player.player_id.0),
                        damage: proj.damage,
                        remaining_health: health.current,
                        was_player: true,
                    },
                });
                // Impact billboard at hit location.
                commands.server_trigger(ToClients {
                    mode: SendMode::Broadcast,
                    message: HitConfirm {
                        target_id: proj.object_id,
                        damage: proj.damage,
                        position: proj.position,
                        kind: carcinisation_net::HitImpactKind::Hit,
                        projectile_type: Some(proj.projectile_type),
                        part_id: None,
                        critical: false,
                    },
                });
            }
            commands.entity(proj_entity).despawn();
            continue;
        }

        // OOB check.
        let gx = next.x.floor() as i32;
        let gy = next.y.floor() as i32;
        if gx < 0 || gy < 0 || gx >= server_map.0.width as i32 || gy >= server_map.0.height as i32 {
            commands.entity(proj_entity).despawn();
            continue;
        }

        // Move forward.
        proj.position = next;
    }
}
