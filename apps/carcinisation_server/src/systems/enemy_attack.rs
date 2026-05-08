//! Server-authoritative Mosquiton ranged and melee attacks.
//!
//! When a Mosquiton is in `NetEnemyState::Attack` and alive, tick a per-entity
//! cooldown. On expiry:
//! - If within melee range: deal direct damage to nearest player.
//! - Otherwise: spawn a `NetProjectile` blood shot aimed at the target.

use bevy::prelude::*;
use bevy_replicon::prelude::*;
use carcinisation_net::{
    DamageEffect, EnemyAttackKind, EnemyAttackVisual, NetEnemy, NetEnemyState, NetEnemyType,
    NetHealth, NetPlayer, NetProjectileType, NetworkObjectId, Owner, PlayerId, PlayerNetState,
};

use carcinisation_fps_core::config::{
    MOSQUITON_ATTACK_INTERVAL, MOSQUITON_MELEE_DAMAGE, MOSQUITON_MELEE_RANGE,
    MOSQUITON_PROJECTILE_DAMAGE, MOSQUITON_SHOOT_CUE_SECS, PROJECTILE_LIFETIME, PROJECTILE_SPEED,
};

use super::NetProjectile;

/// Per-enemy attack cooldown, attached at spawn time.
#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component)]
pub struct EnemyAttackCooldown {
    pub remaining: f32,
    pub interval: f32,
    pub damage: f32,
    pub projectile_speed: f32,
}

impl EnemyAttackCooldown {
    #[must_use]
    pub fn mosquiton() -> Self {
        Self {
            remaining: MOSQUITON_ATTACK_INTERVAL,
            interval: MOSQUITON_ATTACK_INTERVAL,
            damage: MOSQUITON_PROJECTILE_DAMAGE,
            projectile_speed: PROJECTILE_SPEED,
        }
    }
}

/// Delay between shoot animation start and projectile spawn.
/// Uses the authored cue frame timing from the composed atlas.
const SHOOT_LEAD_TIME: f32 = MOSQUITON_SHOOT_CUE_SECS;

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
    #[allow(dead_code)]
    enemy_object_id: NetworkObjectId,
    target_player_id: PlayerId,
    damage: f32,
}

/// Tick enemy attack cooldowns and spawn projectiles or apply melee damage.
#[allow(clippy::too_many_arguments)]
pub fn tick_enemy_attacks(
    mut commands: Commands,
    mut enemies: Query<(Entity, &NetEnemy, &NetHealth, &mut EnemyAttackCooldown)>,
    players: Query<&NetPlayer>,
    mut player_health: Query<(&NetPlayer, &mut NetHealth), Without<NetEnemy>>,
    fixed_time: Res<Time<Fixed>>,
    mut next_id: ResMut<NextProjectileId>,
) {
    let dt = fixed_time.delta_secs();
    let mut melee_hits: Vec<MeleeHit> = Vec::new();

    for (enemy_entity, enemy, health, mut cooldown) in &mut enemies {
        // Only fire when alive and in Attack state.
        if health.current <= 0.0
            || enemy.state != NetEnemyState::HoldingRange
            || enemy.enemy_type != NetEnemyType::Mosquiton
        {
            continue;
        }

        cooldown.remaining -= dt;
        if cooldown.remaining > 0.0 {
            continue;
        }
        cooldown.remaining = cooldown.interval;

        // Find nearest alive player.
        let Some((target_pos, target_pid)) = nearest_alive_player(enemy.position, &players) else {
            continue;
        };

        let to_target = target_pos - enemy.position;
        let dist = to_target.length();
        if dist < 0.01 {
            continue;
        }

        if dist <= MOSQUITON_MELEE_RANGE {
            // Melee: direct damage (deferred to avoid borrow conflict).
            melee_hits.push(MeleeHit {
                enemy_object_id: enemy.object_id,
                target_player_id: target_pid,
                damage: MOSQUITON_MELEE_DAMAGE,
            });
            debug!(
                "Enemy {:?} melee hit player {:?} at dist={:.2}",
                enemy.object_id, target_pid, dist
            );
            // Visual event for melee animation.
            commands.server_trigger(ToClients {
                mode: SendMode::Broadcast,
                message: EnemyAttackVisual {
                    object_id: enemy.object_id,
                    kind: EnemyAttackKind::Melee,
                },
            });
        } else {
            // Ranged: send visual event now, defer projectile spawn so
            // the shoot animation leads the projectile by SHOOT_LEAD_TIME.
            let dir = to_target / dist;
            let angle = dir.y.atan2(dir.x);
            let object_id = next_id.allocate();

            commands.server_trigger(ToClients {
                mode: SendMode::Broadcast,
                message: EnemyAttackVisual {
                    object_id: enemy.object_id,
                    kind: EnemyAttackKind::Ranged,
                },
            });

            commands.spawn(PendingProjectile {
                timer: SHOOT_LEAD_TIME,
                source_entity: enemy_entity,
                position: enemy.position,
                angle,
                damage: cooldown.damage,
                object_id,
            });

            debug!(
                "Enemy {:?} queued projectile {:?} at player angle={:.2}",
                enemy.object_id, object_id, angle
            );
        }
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

/// Spawn deferred projectiles after `SHOOT_LEAD_TIME` expires.
/// Cancels if the source enemy died during the delay.
pub fn tick_pending_projectiles(
    mut commands: Commands,
    mut pending: Query<(Entity, &mut PendingProjectile)>,
    enemies: Query<&NetEnemy>,
    fixed_time: Res<Time<Fixed>>,
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
                super::projectile::ProjectileTtl(PROJECTILE_LIFETIME),
                Replicated,
            ));
            commands.entity(entity).despawn();
        }
    }
}

fn nearest_alive_player(position: Vec2, players: &Query<&NetPlayer>) -> Option<(Vec2, PlayerId)> {
    players
        .iter()
        .filter(|p| matches!(p.state, PlayerNetState::Alive))
        .map(|p| (p.position, p.player_id, p.position.distance(position)))
        .min_by(|(_, pa, a), (_, pb, b)| {
            a.partial_cmp(b)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(pa.0.cmp(&pb.0))
        })
        .map(|(pos, pid, _)| (pos, pid))
}
