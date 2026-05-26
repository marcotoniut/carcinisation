use crate::{data::AnimationData, stage::attack::data::HoveringAttackAnimations};
use bevy::prelude::*;
use carapace::constrained::{FiniteF32, PositiveFiniteF32};
use carapace::prelude::CxAnimationFinishBehavior;
use serde::Deserialize;
use std::time::Duration;

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

/// Tuning parameters for the blood shot projectile.
///
/// Loaded from `assets/config/attacks/blood_shot.ron` via `ron_config!`.
#[derive(Clone, Debug, Deserialize, Resource, Reflect)]
#[reflect(Resource)]
pub struct BloodShotConfig {
    pub depth_speed: FiniteF32,
    pub line_speed: PositiveFiniteF32,
    pub damage: u32,
    pub randomness: FiniteF32,
    pub startup_hold_ms: u64,
}

impl BloodShotConfig {
    /// Load config via `ron_config!` macro (embedded at compile time, with
    /// optional filesystem override when `hot_reload` is enabled).
    ///
    /// # Panics
    ///
    /// Panics if the config fails to parse or validate.
    #[must_use]
    pub fn load() -> Self {
        let config: Self = carcinisation_core::ron_config!("assets/config/attacks/blood_shot.ron");
        config.validate();
        config
    }

    fn validate(&self) {
        assert!(
            self.randomness.get() >= 0.0,
            "BloodShotConfig: randomness must be non-negative, got {}",
            self.randomness,
        );
    }

    /// Startup hold as a [`Duration`].
    #[must_use]
    pub const fn startup_hold(&self) -> Duration {
        Duration::from_millis(self.startup_hold_ms)
    }
}

// ---------------------------------------------------------------------------
// Animations (not config-driven — structural, rarely changes)
// ---------------------------------------------------------------------------

pub static BLOOD_ATTACK_ANIMATIONS: std::sync::LazyLock<HoveringAttackAnimations> =
    std::sync::LazyLock::new(|| {
        let hovering_canonical = AnimationData {
            finish_behavior: CxAnimationFinishBehavior::Loop,
            frames: 4,
            speed: 700,
            ..default()
        };

        let hit_canonical = AnimationData {
            finish_behavior: CxAnimationFinishBehavior::Mark,
            frames: 1,
            speed: 300,
            ..default()
        };

        let destroy_canonical = AnimationData {
            finish_behavior: CxAnimationFinishBehavior::Mark,
            frames: 3,
            speed: 100,
            ..default()
        };

        HoveringAttackAnimations {
            hovering_canonical,
            hit_canonical,
            destroy_canonical: Some(destroy_canonical),
        }
    });

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_config_parses_and_validates() {
        let config = BloodShotConfig::load();
        config.validate();
    }

    #[test]
    fn startup_hold_converts_correctly() {
        let config = BloodShotConfig::load();
        assert_eq!(
            config.startup_hold(),
            Duration::from_millis(config.startup_hold_ms)
        );
    }

    #[test]
    fn values_match_original_constants() {
        let config = BloodShotConfig::load();
        assert!((config.depth_speed.get() - (-5.0)).abs() < f32::EPSILON);
        assert!((config.line_speed.get() - 55.0).abs() < f32::EPSILON);
        assert_eq!(config.damage, 20);
        assert!((config.randomness.get() - 20.0).abs() < f32::EPSILON);
        assert_eq!(config.startup_hold_ms, 60);
    }
}
