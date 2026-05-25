//! Ground fire hazard spawned when an entity dies from burning.
//!
//! Shared headless logic — no rendering, no Bevy ECS.
//! The fps crate wraps this with billboard visuals; the server uses it for
//! authoritative damage.

use bevy_math::Vec2;
use std::num::NonZeroUsize;

use crate::config;
use crate::fire_death::corpse_seed;
use crate::hash_util::unit;

/// A ground fire hazard at a fixed world position.
#[derive(Clone, Debug, PartialEq)]
pub struct GroundFire {
    pub position: Vec2,
    pub remaining_secs: f32,
    pub seed: u32,
}

/// Tuning values for ground fire behavior.
#[derive(Clone, Debug)]
pub struct GroundFireConfig {
    pub lifetime_secs: f32,
    /// Elapsed time at which the fire starts fading (half size, half damage).
    pub fade_start_secs: f32,
    pub radius: f32,
    pub damage_per_tick: f32,
    pub tick_secs: f32,
    pub flame_count: NonZeroUsize,
    pub max_fires: usize,
    /// Visual spread radius for flame billboard placement (tighter than damage radius).
    pub visual_radius: f32,
}

impl Default for GroundFireConfig {
    fn default() -> Self {
        config::FpsCombatConfig::default().ground_fire_config()
    }
}

impl GroundFire {
    /// Returns the elapsed time since this fire was spawned.
    #[must_use]
    pub fn elapsed(&self, config: &GroundFireConfig) -> f32 {
        config.lifetime_secs - self.remaining_secs
    }

    /// Returns `true` if this fire is in its fade phase (past `fade_start_secs`).
    #[must_use]
    pub fn is_fading(&self, config: &GroundFireConfig) -> bool {
        self.elapsed(config) >= config.fade_start_secs
    }

    /// Scale multiplier: 1.0 during full phase, 0.5 during fade phase.
    #[must_use]
    pub fn intensity(&self, config: &GroundFireConfig) -> f32 {
        if self.is_fading(config) { 0.5 } else { 1.0 }
    }
}

/// Spawn a ground fire at `position` if one doesn't already exist nearby
/// and the cap hasn't been reached.
/// Returns `true` if a fire was spawned.
pub fn try_spawn_ground_fire(
    fires: &mut Vec<GroundFire>,
    position: Vec2,
    config: &GroundFireConfig,
) -> bool {
    if fires.len() >= config.max_fires {
        return false;
    }
    // Skip if a fire already exists within a small radius (avoids duplicates
    // from repeated frame checks).
    let dedup_radius_sq = 0.1 * 0.1;
    if fires
        .iter()
        .any(|f| f.position.distance_squared(position) < dedup_radius_sq)
    {
        return false;
    }
    fires.push(GroundFire {
        position,
        remaining_secs: config.lifetime_secs,
        seed: corpse_seed(position),
    });
    true
}

/// Tick ground fire lifetimes and remove expired ones.
pub fn tick_ground_fires(fires: &mut Vec<GroundFire>, dt: f32) {
    for fire in fires.iter_mut() {
        fire.remaining_secs -= dt;
    }
    fires.retain(|f| f.remaining_secs > 0.0);
}

/// Per-player cooldown state for ground fire contact damage.
#[derive(Clone, Debug, Default)]
pub struct GroundFireContactState {
    pub cooldown_remaining_secs: f32,
}

/// Result of ground fire contact damage check for a single player.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GroundFireContactResult {
    pub player_damage: u32,
    pub damage_source: Option<Vec2>,
}

/// Check ground fire contact damage against a player position.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn ground_fire_contact_damage(
    player_pos: Vec2,
    fires: &[GroundFire],
    config: &GroundFireConfig,
    state: &mut GroundFireContactState,
    dt: f32,
) -> GroundFireContactResult {
    state.cooldown_remaining_secs = (state.cooldown_remaining_secs - dt).max(0.0);

    if config.damage_per_tick <= 0.0 || config.radius <= 0.0 || fires.is_empty() {
        return GroundFireContactResult::default();
    }

    if state.cooldown_remaining_secs > 0.0 {
        return GroundFireContactResult::default();
    }

    let radius_sq = config.radius * config.radius;
    let closest = fires
        .iter()
        .filter_map(|f| {
            let dist_sq = f.position.distance_squared(player_pos);
            (dist_sq <= radius_sq).then_some((f, dist_sq))
        })
        .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let Some((fire, _)) = closest else {
        return GroundFireContactResult::default();
    };

    state.cooldown_remaining_secs = config.tick_secs;
    let damage = config.damage_per_tick * fire.intensity(config);
    GroundFireContactResult {
        player_damage: damage as u32,
        damage_source: Some(fire.position),
    }
}

/// Deterministic flame placement for ground fire visuals.
/// Returns `(lateral_offset, forward_offset, scale, phase_secs)` per flame.
#[must_use]
pub fn ground_fire_flame_layout(seed: u32, count: usize, radius: f32) -> Vec<(Vec2, f32, f32)> {
    (0..count)
        .map(|i| {
            let mixed = seed ^ (i as u32).wrapping_mul(0x9E37_79B9);
            let angle = unit(mixed.rotate_left(3)) * std::f32::consts::TAU;
            let r = unit(mixed.rotate_left(11)).sqrt() * radius;
            let offset = Vec2::new(angle.cos() * r, angle.sin() * r);
            let scale = 0.7 + unit(mixed.rotate_left(17)) * 0.6;
            let phase = unit(mixed.rotate_left(7)) * 0.3;
            (offset, scale, phase)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> GroundFireConfig {
        GroundFireConfig::default()
    }

    #[test]
    fn spawn_adds_fire_at_position() {
        let mut fires = Vec::new();
        let config = default_config();
        let spawned = try_spawn_ground_fire(&mut fires, Vec2::new(3.0, 4.0), &config);
        assert!(spawned);
        assert_eq!(fires.len(), 1);
        assert_eq!(fires[0].position, Vec2::new(3.0, 4.0));
        assert!((fires[0].remaining_secs - config.lifetime_secs).abs() < 1e-5);
    }

    #[test]
    fn spawn_deduplicates_nearby_positions() {
        let mut fires = Vec::new();
        let config = default_config();
        try_spawn_ground_fire(&mut fires, Vec2::new(3.0, 4.0), &config);
        let spawned = try_spawn_ground_fire(&mut fires, Vec2::new(3.05, 4.05), &config);
        assert!(!spawned);
        assert_eq!(fires.len(), 1);
    }

    #[test]
    fn spawn_allows_distant_positions() {
        let mut fires = Vec::new();
        let config = default_config();
        try_spawn_ground_fire(&mut fires, Vec2::new(3.0, 4.0), &config);
        let spawned = try_spawn_ground_fire(&mut fires, Vec2::new(5.0, 6.0), &config);
        assert!(spawned);
        assert_eq!(fires.len(), 2);
    }

    #[test]
    fn spawn_respects_max_cap() {
        let config = GroundFireConfig {
            max_fires: 2,
            ..default_config()
        };
        let mut fires = Vec::new();
        try_spawn_ground_fire(&mut fires, Vec2::new(1.0, 1.0), &config);
        try_spawn_ground_fire(&mut fires, Vec2::new(2.0, 2.0), &config);
        let spawned = try_spawn_ground_fire(&mut fires, Vec2::new(3.0, 3.0), &config);
        assert!(!spawned);
        assert_eq!(fires.len(), 2);
    }

    #[test]
    fn tick_decrements_lifetime_and_removes_expired() {
        let mut fires = vec![
            GroundFire {
                position: Vec2::new(1.0, 1.0),
                remaining_secs: 0.5,
                seed: 1,
            },
            GroundFire {
                position: Vec2::new(2.0, 2.0),
                remaining_secs: 2.0,
                seed: 2,
            },
        ];
        tick_ground_fires(&mut fires, 0.6);
        assert_eq!(fires.len(), 1);
        assert_eq!(fires[0].seed, 2);
        assert!((fires[0].remaining_secs - 1.4).abs() < 1e-5);
    }

    #[test]
    fn contact_damage_applies_within_radius() {
        let config = default_config();
        let fires = vec![GroundFire {
            position: Vec2::new(3.0, 3.0),
            remaining_secs: 2.0,
            seed: 1,
        }];
        let mut state = GroundFireContactState::default();
        let result =
            ground_fire_contact_damage(Vec2::new(3.3, 3.0), &fires, &config, &mut state, 0.016);
        assert!(result.player_damage > 0);
        assert_eq!(result.damage_source, Some(Vec2::new(3.0, 3.0)));
    }

    #[test]
    fn contact_damage_skips_outside_radius() {
        let config = default_config();
        let fires = vec![GroundFire {
            position: Vec2::new(3.0, 3.0),
            remaining_secs: 2.0,
            seed: 1,
        }];
        let mut state = GroundFireContactState::default();
        let result =
            ground_fire_contact_damage(Vec2::new(10.0, 10.0), &fires, &config, &mut state, 0.016);
        assert_eq!(result.player_damage, 0);
        assert!(result.damage_source.is_none());
    }

    #[test]
    fn contact_damage_respects_cooldown() {
        let config = default_config();
        let fires = vec![GroundFire {
            position: Vec2::new(3.0, 3.0),
            remaining_secs: 2.0,
            seed: 1,
        }];
        let mut state = GroundFireContactState::default();
        // First tick — damage.
        let r1 =
            ground_fire_contact_damage(Vec2::new(3.0, 3.0), &fires, &config, &mut state, 0.016);
        assert!(r1.player_damage > 0);
        // Immediate second tick — cooldown blocks.
        let r2 =
            ground_fire_contact_damage(Vec2::new(3.0, 3.0), &fires, &config, &mut state, 0.016);
        assert_eq!(r2.player_damage, 0);
    }

    #[test]
    fn contact_damage_fires_again_after_cooldown() {
        let config = default_config();
        let fires = vec![GroundFire {
            position: Vec2::new(3.0, 3.0),
            remaining_secs: 5.0,
            seed: 1,
        }];
        let mut state = GroundFireContactState::default();
        ground_fire_contact_damage(Vec2::new(3.0, 3.0), &fires, &config, &mut state, 0.016);
        // Wait out cooldown.
        let r = ground_fire_contact_damage(
            Vec2::new(3.0, 3.0),
            &fires,
            &config,
            &mut state,
            config.tick_secs + 0.01,
        );
        assert!(r.player_damage > 0);
    }

    #[test]
    fn flame_layout_is_deterministic() {
        let a = ground_fire_flame_layout(42, 6, 0.8);
        let b = ground_fire_flame_layout(42, 6, 0.8);
        assert_eq!(a.len(), b.len());
        for (fa, fb) in a.iter().zip(b.iter()) {
            assert!((fa.0 - fb.0).length() < 1e-6);
            assert!((fa.1 - fb.1).abs() < 1e-6);
            assert!((fa.2 - fb.2).abs() < 1e-6);
        }
    }

    #[test]
    fn flame_layout_positions_within_radius() {
        let radius = 0.8;
        let flames = ground_fire_flame_layout(99, 10, radius);
        for (offset, _, _) in &flames {
            assert!(
                offset.length() <= radius + 1e-5,
                "flame at {offset:?} exceeds radius {radius}"
            );
        }
    }

    #[test]
    fn empty_fires_no_contact_damage() {
        let config = default_config();
        let mut state = GroundFireContactState::default();
        let result =
            ground_fire_contact_damage(Vec2::new(3.0, 3.0), &[], &config, &mut state, 0.016);
        assert_eq!(result.player_damage, 0);
    }
}
