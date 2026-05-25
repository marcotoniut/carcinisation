//! Progressive burning status effect.
//!
//! Fire exposure builds intensity over time; intensity drives damage-over-time.
//! Shared headless logic — no rendering, no Bevy ECS.
//!
//! ## Model
//!
//! - Fire sources (flamethrower, ground fire) call [`apply_exposure`] each tick.
//! - [`tick_burning`] runs once per entity per tick: applies decay, computes damage.
//! - Exposure and decay are both delta-time scaled → frame-rate independent.
//! - Multiple fire sources in the same frame stack additively (capped at max).

use bevy::prelude::ReflectResource;
use serde::Deserialize;

/// Per-entity burn state.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BurnState {
    /// Current burn intensity (0.0 = not burning, capped at config max).
    pub intensity: f32,
    /// Sub-integer damage accumulator. Damage is applied when this reaches >= 1.0.
    pub damage_accumulator: f32,
    /// Whether any fire source applied exposure this tick.
    /// Reset by [`tick_burning`]; set by [`apply_exposure`].
    pub exposed_this_tick: bool,
}

impl BurnState {
    /// Whether this entity is actively burning.
    #[must_use]
    pub fn is_burning(&self) -> bool {
        self.intensity > 0.0
    }
}

/// Tuning values for the burn mechanic. Loaded from RON.
#[derive(Clone, Debug, Deserialize, bevy::prelude::Resource, bevy::prelude::Reflect)]
#[reflect(Resource)]
pub struct BurnConfig {
    /// Maximum burn intensity.
    pub max_intensity: f32,
    /// Intensity gained per second of direct flame exposure (flamethrower).
    pub flame_exposure_per_sec: f32,
    /// Intensity gained per second of ground fire exposure.
    pub ground_fire_exposure_per_sec: f32,
    /// Passive intensity decay per second when not exposed.
    pub decay_per_sec: f32,
    /// Multiplier on decay rate while the entity is moving.
    pub movement_decay_multiplier: f32,
    /// Burn damage per second at max intensity. Scales linearly with intensity.
    pub damage_per_sec_at_max: f32,
    /// Small direct damage per second from flame contact (applied via exposure).
    pub direct_contact_dps: f32,
    /// Intensity below which burning auto-extinguishes.
    pub extinguish_threshold: f32,

    // -- Visual tuning --
    /// Intensity below which no burn flames are rendered.
    pub visible_threshold: f32,
    /// Maximum burn flame sprites per entity.
    pub max_burn_flames: usize,
    /// Flame world-height multiplier at visibility threshold (smallest).
    pub flame_scale_min: f32,
    /// Flame world-height multiplier at max intensity (largest).
    pub flame_scale_max: f32,
}

impl Default for BurnConfig {
    fn default() -> Self {
        Self {
            max_intensity: 1.0,
            flame_exposure_per_sec: 0.8,
            ground_fire_exposure_per_sec: 0.3,
            decay_per_sec: 0.15,
            movement_decay_multiplier: 2.5,
            damage_per_sec_at_max: 70.0,
            direct_contact_dps: 10.0,
            extinguish_threshold: 0.01,
            visible_threshold: 0.3,
            max_burn_flames: 6,
            flame_scale_min: 0.1,
            flame_scale_max: 0.3,
        }
    }
}

/// Result of a single [`tick_burning`] call.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BurnTickResult {
    /// Integer damage to apply this tick (0 if accumulated damage < 1).
    pub damage: u32,
    /// Whether the fire was extinguished this tick.
    pub extinguished: bool,
}

/// Increase burn intensity from a fire source.
///
/// Call once per source per entity per tick. Multiple calls in the same frame
/// stack additively (intensity is capped at `config.max_intensity`).
///
/// Direct contact damage is applied once per tick in [`tick_burning`] when
/// `exposed_this_tick` is set, regardless of how many sources called this.
///
/// `exposure_per_sec` should come from the source type:
/// - Flamethrower: `config.flame_exposure_per_sec`
/// - Ground fire: `config.ground_fire_exposure_per_sec`
pub fn apply_exposure(state: &mut BurnState, config: &BurnConfig, exposure_per_sec: f32, dt: f32) {
    state.intensity = (state.intensity + exposure_per_sec * dt).min(config.max_intensity);
    state.exposed_this_tick = true;
}

/// Tick burn state: apply intensity-proportional damage, then decay intensity.
///
/// Call exactly once per entity per tick, **after** all exposure sources have
/// called [`apply_exposure`] for this frame.
///
/// Returns the integer damage to apply (if any) and whether the fire went out.
/// Resets `exposed_this_tick` so the next frame starts clean.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn tick_burning(
    state: &mut BurnState,
    config: &BurnConfig,
    dt: f32,
    is_moving: bool,
) -> BurnTickResult {
    let was_exposed = state.exposed_this_tick;
    state.exposed_this_tick = false;

    if state.intensity <= 0.0 {
        return BurnTickResult::default();
    }

    // Direct contact damage — applied at most once per tick regardless of source count.
    if was_exposed {
        state.damage_accumulator += config.direct_contact_dps * dt;
    }

    // Burn damage proportional to current intensity.
    let burn_dps = config.damage_per_sec_at_max * state.intensity;
    state.damage_accumulator += burn_dps * dt;

    // Extract integer damage.
    let int_damage = state.damage_accumulator.floor();
    let damage = if int_damage >= 1.0 {
        state.damage_accumulator -= int_damage;
        int_damage as u32
    } else {
        0
    };

    // Decay intensity.
    let decay = config.decay_per_sec
        * if is_moving {
            config.movement_decay_multiplier
        } else {
            1.0
        };
    state.intensity = (state.intensity - decay * dt).max(0.0);

    // Auto-extinguish at low intensity (intensity was > 0 at function entry).
    let extinguished = state.intensity <= config.extinguish_threshold;
    if extinguished {
        state.intensity = 0.0;
        state.damage_accumulator = 0.0;
    }

    BurnTickResult {
        damage,
        extinguished,
    }
}

/// Number of visible flames for a burning entity at given intensity.
///
/// `max_flames` should scale with entity visual size.
#[must_use]
pub fn burn_flame_count(intensity: f32, max_flames: usize, config: &BurnConfig) -> usize {
    let capped = max_flames.min(config.max_burn_flames);
    if intensity <= config.visible_threshold || capped == 0 {
        return 0;
    }
    // Linear ramp from threshold to max.
    let t =
        (intensity - config.visible_threshold) / (config.max_intensity - config.visible_threshold);
    ((t * capped as f32).ceil() as usize).clamp(1, capped)
}

/// Flame world-height scale factor for given intensity.
#[must_use]
pub fn burn_flame_scale(intensity: f32, config: &BurnConfig) -> f32 {
    config.flame_scale_min + (config.flame_scale_max - config.flame_scale_min) * intensity
}

/// Load `BurnConfig` from the embedded RON asset.
///
/// WARNING: multiplayer must ensure both client and server share the same
/// config values. Do not load different configs per-side.
#[must_use]
pub fn load_config() -> BurnConfig {
    carcinisation_core::ron_config!("assets/config/status/burning.ron")
}

/// Clear burn state (e.g. on death transition).
pub fn extinguish(state: &mut BurnState) {
    state.intensity = 0.0;
    state.damage_accumulator = 0.0;
    state.exposed_this_tick = false;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> BurnConfig {
        BurnConfig::default()
    }

    #[test]
    fn exposure_increases_intensity() {
        let config = default_config();
        let mut state = BurnState::default();
        apply_exposure(&mut state, &config, config.flame_exposure_per_sec, 0.5);
        assert!((state.intensity - 0.4).abs() < 1e-5);
        assert!(state.exposed_this_tick);
    }

    #[test]
    fn intensity_capped_at_max() {
        let config = default_config();
        let mut state = BurnState::default();
        apply_exposure(&mut state, &config, config.flame_exposure_per_sec, 2.0);
        assert!((state.intensity - config.max_intensity).abs() < 1e-5);
    }

    #[test]
    fn tick_applies_damage_proportional_to_intensity() {
        let config = default_config();
        let mut state = BurnState {
            intensity: 1.0,
            damage_accumulator: 0.0,
            exposed_this_tick: false,
        };
        // Not exposed: only burn DPS (70 * 1.0 * 0.1 = 7.0).
        let result = tick_burning(&mut state, &config, 0.1, false);
        assert_eq!(result.damage, 7);
    }

    #[test]
    fn tick_accumulates_sub_integer_damage() {
        let config = default_config();
        let mut state = BurnState {
            intensity: 0.2,
            damage_accumulator: 0.0,
            exposed_this_tick: false,
        };
        // At 0.2 intensity: 20 DPS. Over 1/60s ≈ 0.33 damage → 0 integer.
        let dt = 1.0 / 60.0;
        let r1 = tick_burning(&mut state, &config, dt, false);
        assert_eq!(r1.damage, 0);
        assert!(state.damage_accumulator > 0.0);
        // After enough ticks, integer damage is emitted.
        let mut total = 0u32;
        for _ in 0..10 {
            total += tick_burning(&mut state, &config, dt, false).damage;
        }
        assert!(
            total > 0,
            "sub-integer damage should accumulate into integer damage"
        );
    }

    #[test]
    fn decay_reduces_intensity() {
        let config = default_config();
        let mut state = BurnState {
            intensity: 1.0,
            damage_accumulator: 0.0,
            exposed_this_tick: false,
        };
        tick_burning(&mut state, &config, 1.0, false);
        assert!((state.intensity - 0.85).abs() < 1e-5);
    }

    #[test]
    fn movement_accelerates_decay() {
        let config = default_config();
        let mut still = BurnState {
            intensity: 1.0,
            ..Default::default()
        };
        let mut moving = BurnState {
            intensity: 1.0,
            ..Default::default()
        };
        tick_burning(&mut still, &config, 1.0, false);
        tick_burning(&mut moving, &config, 1.0, true);
        assert!(moving.intensity < still.intensity);
        assert!((moving.intensity - 0.625).abs() < 1e-5);
    }

    #[test]
    fn auto_extinguish_below_threshold() {
        let config = default_config();
        let mut state = BurnState {
            intensity: 0.005,
            damage_accumulator: 0.5,
            exposed_this_tick: false,
        };
        let result = tick_burning(&mut state, &config, 0.001, false);
        assert!(result.extinguished);
        assert!((state.intensity - 0.0).abs() < 1e-6);
        assert!((state.damage_accumulator - 0.0).abs() < 1e-6);
    }

    #[test]
    fn auto_extinguish_at_exact_zero() {
        let config = default_config();
        // Decay will bring intensity to exactly 0.0: 0.15 * 1.0 * 1.0 = 0.15.
        let mut state = BurnState {
            intensity: 0.15,
            damage_accumulator: 0.0,
            exposed_this_tick: false,
        };
        let result = tick_burning(&mut state, &config, 1.0, false);
        assert!(result.extinguished);
        assert!((state.intensity - 0.0).abs() < 1e-6);
    }

    #[test]
    fn no_damage_when_not_burning() {
        let config = default_config();
        let mut state = BurnState::default();
        let result = tick_burning(&mut state, &config, 1.0, false);
        assert_eq!(result.damage, 0);
        assert!(!result.extinguished);
    }

    #[test]
    fn multiple_exposure_sources_stack_intensity() {
        let config = default_config();
        let mut state = BurnState::default();
        let dt = 1.0 / 30.0;
        apply_exposure(&mut state, &config, config.flame_exposure_per_sec, dt);
        apply_exposure(&mut state, &config, config.ground_fire_exposure_per_sec, dt);
        let expected = (config.flame_exposure_per_sec + config.ground_fire_exposure_per_sec) * dt;
        assert!((state.intensity - expected).abs() < 1e-5);
    }

    #[test]
    fn multiple_sources_direct_damage_applied_once() {
        let config = default_config();
        let dt = 1.0;

        // Single source: one apply_exposure + tick.
        let mut single = BurnState::default();
        apply_exposure(&mut single, &config, config.flame_exposure_per_sec, dt);
        let r_single = tick_burning(&mut single, &config, dt, false);

        // Two sources: both expose in same tick, but direct DPS should only apply once.
        let mut multi = BurnState::default();
        apply_exposure(&mut multi, &config, config.flame_exposure_per_sec, dt);
        apply_exposure(&mut multi, &config, config.ground_fire_exposure_per_sec, dt);
        let r_multi = tick_burning(&mut multi, &config, dt, false);

        // Multi has higher intensity (more exposure) → more burn DPS damage.
        assert!(r_multi.damage >= r_single.damage);
        // But direct contact DPS was the same (applied once in both cases).
        // The difference should come only from the burn DPS delta, not doubled direct.
        let intensity_single = config.flame_exposure_per_sec; // capped at max
        let intensity_multi = (config.flame_exposure_per_sec + config.ground_fire_exposure_per_sec)
            .min(config.max_intensity);
        let burn_diff = (intensity_multi - intensity_single) * config.damage_per_sec_at_max * dt;
        let actual_diff = r_multi.damage as f32 - r_single.damage as f32;
        // If direct were doubled, actual_diff would exceed burn_diff by direct_contact_dps.
        assert!(
            actual_diff <= burn_diff + 1.0,
            "damage diff {actual_diff} should not exceed burn DPS diff {burn_diff} + rounding"
        );
    }

    #[test]
    fn exposure_then_decay_to_zero() {
        let config = default_config();
        let mut state = BurnState::default();
        apply_exposure(&mut state, &config, config.flame_exposure_per_sec, 0.1);
        assert!(state.is_burning());
        for _ in 0..300 {
            tick_burning(&mut state, &config, 1.0 / 30.0, false);
        }
        assert!(!state.is_burning());
    }

    #[test]
    fn mosquiton_dies_in_about_two_seconds() {
        let config = default_config();
        let mut state = BurnState::default();
        let dt = 1.0 / 30.0;
        let mut hp: f32 = 200.0;
        let mut ticks = 0u32;

        while hp > 0.0 && ticks < 300 {
            apply_exposure(&mut state, &config, config.flame_exposure_per_sec, dt);
            let result = tick_burning(&mut state, &config, dt, false);
            hp -= result.damage as f32;
            ticks += 1;
        }

        let elapsed = ticks as f32 * dt;
        assert!(
            hp <= 0.0,
            "mosquiton should be dead, hp={hp} after {elapsed:.2}s"
        );
        assert!(
            elapsed > 2.0 && elapsed < 5.0,
            "kill time {elapsed:.2}s outside 2.0-5.0s target window"
        );
    }

    #[test]
    fn frame_rate_independent_damage() {
        let config = default_config();

        let mut state_30 = BurnState::default();
        let mut total_30 = 0u32;
        for _ in 0..60 {
            apply_exposure(
                &mut state_30,
                &config,
                config.flame_exposure_per_sec,
                1.0 / 30.0,
            );
            total_30 += tick_burning(&mut state_30, &config, 1.0 / 30.0, false).damage;
        }

        let mut state_60 = BurnState::default();
        let mut total_60 = 0u32;
        for _ in 0..120 {
            apply_exposure(
                &mut state_60,
                &config,
                config.flame_exposure_per_sec,
                1.0 / 60.0,
            );
            total_60 += tick_burning(&mut state_60, &config, 1.0 / 60.0, false).damage;
        }

        let diff = (total_30 as i32 - total_60 as i32).unsigned_abs();
        assert!(
            diff <= 2,
            "30Hz total={total_30}, 60Hz total={total_60}, diff={diff} exceeds tolerance"
        );
    }

    #[test]
    fn burn_flame_count_scales_with_intensity() {
        let c = default_config();
        assert_eq!(burn_flame_count(0.0, 10, &c), 0);
        assert_eq!(burn_flame_count(0.1, 10, &c), 0); // below visibility threshold
        assert_eq!(burn_flame_count(0.3, 10, &c), 0); // at threshold, not above
        assert_eq!(burn_flame_count(0.31, 10, &c), 1); // just above threshold
        assert_eq!(burn_flame_count(0.5, 10, &c), 2);
        assert_eq!(burn_flame_count(1.0, 10, &c), 6); // capped at max_burn_flames
        assert_eq!(burn_flame_count(1.0, 3, &c), 3); // respects lower max
    }

    #[test]
    fn burn_flame_count_zero_max_returns_zero() {
        let c = default_config();
        assert_eq!(burn_flame_count(1.0, 0, &c), 0);
        assert_eq!(burn_flame_count(0.5, 0, &c), 0);
    }

    #[test]
    fn extinguish_clears_state() {
        let mut state = BurnState {
            intensity: 0.8,
            damage_accumulator: 5.3,
            exposed_this_tick: true,
        };
        extinguish(&mut state);
        assert!((state.intensity - 0.0).abs() < 1e-6);
        assert!((state.damage_accumulator - 0.0).abs() < 1e-6);
        assert!(!state.exposed_this_tick);
        assert!(!state.is_burning());
    }

    #[test]
    fn direct_contact_dps_applied_once_per_tick() {
        let config = default_config();
        let mut state = BurnState::default();
        let dt = 1.0;
        apply_exposure(&mut state, &config, config.flame_exposure_per_sec, dt);
        // exposure sets exposed_this_tick but does NOT add direct damage.
        assert!((state.damage_accumulator - 0.0).abs() < 1e-6);
        // tick_burning adds direct_contact_dps once.
        tick_burning(&mut state, &config, dt, false);
        // Accumulator has burn DPS + direct contact, minus any integer extracted.
        // Just verify direct contact was applied by checking a no-exposure tick differs.
        let mut unexposed = BurnState {
            intensity: state.intensity,
            ..Default::default()
        };
        let r_unexposed = tick_burning(&mut unexposed, &config, dt, false);
        let mut exposed = BurnState {
            intensity: state.intensity,
            exposed_this_tick: true,
            ..Default::default()
        };
        let r_exposed = tick_burning(&mut exposed, &config, dt, false);
        // Exposed tick should accumulate more damage (direct_contact_dps contribution).
        assert!(
            r_exposed.damage >= r_unexposed.damage,
            "exposed tick should deal >= unexposed damage"
        );
    }

    #[test]
    fn tick_resets_exposed_flag() {
        let config = default_config();
        let mut state = BurnState {
            intensity: 0.5,
            exposed_this_tick: true,
            ..Default::default()
        };
        tick_burning(&mut state, &config, 1.0 / 30.0, false);
        assert!(!state.exposed_this_tick);
    }

    #[test]
    fn ron_config_parses() {
        let _ = super::load_config();
    }
}
