//! Deterministic simulation state hashing.
//!
//! Produces a u64 hash of all gameplay-critical replicated state in a single
//! tick. Used by replay tests to verify deterministic simulation — identical
//! inputs must produce identical hashes.
//!
//! # Ordering
//!
//! Entities are sorted by `PlayerId`/`NetworkObjectId` before hashing to
//! produce a stable hash regardless of ECS archetype iteration order.

use crate::components::{NetEnemy, NetHealth, NetPlayer, NetProjectile};

/// FNV-1a 64-bit hash of the simulation state visible to all clients.
///
/// Hashes: player positions/angles/states, enemy positions/angles/states,
/// projectile positions/angles, health values.
#[must_use]
pub fn compute_sim_hash(
    players: &[(NetPlayer, NetHealth)],
    enemies: &[(NetEnemy, f32)],
    projectiles: &[NetProjectile],
) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325_u64; // FNV offset basis

    // Players — sorted by PlayerId for stable ordering.
    let mut sorted_players: Vec<_> = players.to_vec();
    sorted_players.sort_by_key(|(p, _)| p.player_id.0);
    for (p, health) in &sorted_players {
        h = fnv_u32(h, p.player_id.0);
        h = fnv_f32(h, p.position.x);
        h = fnv_f32(h, p.position.y);
        h = fnv_f32(h, p.angle);
        h = fnv_u8(h, p.current_attack as u8);
        h = fnv_u8(
            h,
            match p.state {
                crate::PlayerNetState::Alive => 0,
                crate::PlayerNetState::Dead => 1,
            },
        );
        h = fnv_u8(h, u8::from(p.flame_active));
        h = fnv_f32(h, health.current);
    }

    // Enemies — sorted by NetworkObjectId.
    let mut sorted_enemies: Vec<_> = enemies.to_vec();
    sorted_enemies.sort_by_key(|(e, _)| e.object_id.0);
    for (e, health) in &sorted_enemies {
        h = fnv_u32(h, e.object_id.0);
        h = fnv_f32(h, e.position.x);
        h = fnv_f32(h, e.position.y);
        h = fnv_f32(h, e.angle);
        h = fnv_f32(h, *health);
        h = fnv_u8(
            h,
            match e.state {
                crate::NetEnemyState::Idle => 0,
                crate::NetEnemyState::Chase => 1,
                crate::NetEnemyState::HoldingRange => 2,
                crate::NetEnemyState::Dying { burn: false } => 3,
                crate::NetEnemyState::Dying { burn: true } => 4,
                crate::NetEnemyState::Dead { burn: false } => 5,
                crate::NetEnemyState::Dead { burn: true } => 6,
            },
        );
        h = fnv_u8(
            h,
            match e.enemy_type {
                crate::NetEnemyType::Basic => 0,
                crate::NetEnemyType::Mosquiton => 1,
                crate::NetEnemyType::Spidey => 2,
            },
        );
    }

    // Projectiles — sorted by NetworkObjectId.
    let mut sorted_projs: Vec<_> = projectiles.to_vec();
    sorted_projs.sort_by_key(|p| p.object_id.0);
    for p in &sorted_projs {
        h = fnv_u32(h, p.object_id.0);
        h = fnv_f32(h, p.position.x);
        h = fnv_f32(h, p.position.y);
        h = fnv_f32(h, p.angle);
        h = fnv_f32(h, p.damage);
        h = fnv_u32(h, p.owner.0.0);
        h = fnv_u8(
            h,
            match p.projectile_type {
                crate::NetProjectileType::BloodShot => 0,
                crate::NetProjectileType::WebShot => 1,
            },
        );
    }

    h
}

/// Collect simulation state from Bevy queries into vecs suitable for hashing.
///
/// This is a convenience for test code that has query results.
#[must_use]
pub fn collect_player_state(players: &[(&NetPlayer, &NetHealth)]) -> Vec<(NetPlayer, NetHealth)> {
    players
        .iter()
        .map(|(p, h)| ((*p).clone(), (*h).clone()))
        .collect()
}

#[must_use]
pub fn collect_enemy_state(enemies: &[(&NetEnemy, &NetHealth)]) -> Vec<(NetEnemy, f32)> {
    enemies
        .iter()
        .map(|(e, h)| ((*e).clone(), h.current))
        .collect()
}

// Note: NetHealth.max is intentionally omitted from the hash — it is
// constant during gameplay. If a future feature modifies max mid-game,
// it should be added here.

// --- FNV-1a primitives ---

const FNV_PRIME: u64 = 0x0100_0000_01b3;

fn fnv_u8(hash: u64, val: u8) -> u64 {
    (hash ^ u64::from(val)).wrapping_mul(FNV_PRIME)
}

fn fnv_u32(hash: u64, val: u32) -> u64 {
    let bytes = val.to_le_bytes();
    let mut h = hash;
    for b in bytes {
        h = fnv_u8(h, b);
    }
    h
}

fn fnv_f32(hash: u64, val: f32) -> u64 {
    fnv_u32(hash, val.to_bits())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::*;
    use crate::protocol::*;
    use bevy_math::Vec2;

    #[test]
    fn identical_state_produces_identical_hash() {
        let players = vec![(
            NetPlayer {
                player_id: PlayerId(1),
                position: Vec2::new(1.5, 2.5),
                angle: 0.5,
                current_attack: NetAttackId::None,
                state: PlayerNetState::Alive,
                flame_active: false,
            },
            NetHealth {
                current: 100.0,
                max: 100.0,
            },
        )];
        let enemies = vec![];
        let projs = vec![];

        let h1 = compute_sim_hash(&players, &enemies, &projs);
        let h2 = compute_sim_hash(&players, &enemies, &projs);
        assert_eq!(h1, h2);
    }

    #[test]
    fn different_position_produces_different_hash() {
        let make = |x: f32| {
            vec![(
                NetPlayer {
                    player_id: PlayerId(1),
                    position: Vec2::new(x, 2.5),
                    angle: 0.5,
                    current_attack: NetAttackId::None,
                    state: PlayerNetState::Alive,
                    flame_active: false,
                },
                NetHealth {
                    current: 100.0,
                    max: 100.0,
                },
            )]
        };
        let h1 = compute_sim_hash(&make(1.0), &[], &[]);
        let h2 = compute_sim_hash(&make(2.0), &[], &[]);
        assert_ne!(h1, h2);
    }

    #[test]
    fn order_independent_hash() {
        let p1 = (
            NetPlayer {
                player_id: PlayerId(1),
                position: Vec2::new(1.0, 1.0),
                angle: 0.0,
                current_attack: NetAttackId::None,
                state: PlayerNetState::Alive,
                flame_active: false,
            },
            NetHealth {
                current: 100.0,
                max: 100.0,
            },
        );
        let p2 = (
            NetPlayer {
                player_id: PlayerId(2),
                position: Vec2::new(2.0, 2.0),
                angle: 1.0,
                current_attack: NetAttackId::Projectile,
                state: PlayerNetState::Alive,
                flame_active: true,
            },
            NetHealth {
                current: 50.0,
                max: 100.0,
            },
        );

        let h_forward = compute_sim_hash(&[p1.clone(), p2.clone()], &[], &[]);
        let h_reverse = compute_sim_hash(&[p2, p1], &[], &[]);
        assert_eq!(h_forward, h_reverse, "hash should be order-independent");
    }
}
