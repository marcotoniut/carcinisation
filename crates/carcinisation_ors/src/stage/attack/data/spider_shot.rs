use crate::{data::AnimationData, stage::attack::data::HoveringAttackAnimations};
use bevy::prelude::*;
use carapace::constrained::{FiniteF32, PositiveFiniteF32};
use carapace::prelude::CxAnimationFinishBehavior;
use serde::Deserialize;
use std::time::Duration;

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

/// Tuning parameters for the spider shot projectile.
///
/// Loaded from `assets/config/attacks/spider_shot.ron` via `ron_config!`.
#[derive(Clone, Debug, Deserialize, Resource, Reflect)]
#[reflect(Resource)]
pub struct SpiderShotConfig {
    pub depth_speed: FiniteF32,
    /// Retained for future use / tuning reference. The current spawn uses
    /// time-matched velocity (displacement / `depth_time`) instead of a fixed
    /// lateral speed, so this field is not read at runtime.
    pub line_speed: PositiveFiniteF32,
    pub damage: u32,
    pub randomness: FiniteF32,
    pub startup_hold_ms: u64,
}

impl SpiderShotConfig {
    /// How long the projectile stays at its spawn point before beginning travel.
    #[must_use]
    pub const fn startup_hold(&self) -> Duration {
        Duration::from_millis(self.startup_hold_ms)
    }
}

impl SpiderShotConfig {
    #[must_use]
    pub fn load() -> Self {
        let config: Self = carcinisation_core::ron_config!("assets/config/attacks/spider_shot.ron");
        config.validate();
        config
    }

    fn validate(&self) {
        assert!(
            self.randomness.get() >= 0.0,
            "SpiderShotConfig: randomness must be non-negative, got {}",
            self.randomness,
        );
    }
}

// ---------------------------------------------------------------------------
// Animations
// ---------------------------------------------------------------------------

pub static SPIDER_ATTACK_ANIMATIONS: std::sync::LazyLock<HoveringAttackAnimations> =
    std::sync::LazyLock::new(|| {
        // TODO: These are fallback values. Animation params are now data-driven
        // from the atlas RON. These only apply if the atlas is not loaded yet.
        let hovering_canonical = AnimationData {
            finish_behavior: CxAnimationFinishBehavior::Loop,
            frames: 1,
            speed: 300,
            ..default()
        };

        let hit_canonical = AnimationData {
            finish_behavior: CxAnimationFinishBehavior::Mark,
            frames: 1,
            speed: 100,
            ..default()
        };

        let destroy_canonical = AnimationData {
            finish_behavior: CxAnimationFinishBehavior::Mark,
            frames: 1,
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
        let config = SpiderShotConfig::load();
        config.validate();
    }

    #[test]
    fn values_match_original_constants() {
        let config = SpiderShotConfig::load();
        assert!((config.depth_speed.get() - (-4.0)).abs() < f32::EPSILON);
        assert!((config.line_speed.get() - 45.0).abs() < f32::EPSILON);
        assert_eq!(config.damage, 5);
        assert!((config.randomness.get() - 15.0).abs() < f32::EPSILON);
        assert_eq!(config.startup_hold_ms, 80);
    }
}
