use crate::{data::AnimationData, stage::attack::data::HoveringAttackAnimations};
use bevy::prelude::*;
use carapace::constrained::FiniteF32;
use carapace::prelude::CxAnimationFinishBehavior;
use serde::Deserialize;

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

/// Tuning parameters for the boulder throw projectile.
///
/// Loaded from `assets/config/attacks/boulder_throw.ron` via `ron_config!`.
#[derive(Clone, Debug, Deserialize, Resource, Reflect)]
#[reflect(Resource)]
pub struct BoulderThrowConfig {
    pub depth_speed: FiniteF32,
    pub line_y_acceleration: FiniteF32,
    pub damage: u32,
    pub randomness: FiniteF32,
}

impl BoulderThrowConfig {
    #[must_use]
    pub fn load() -> Self {
        carcinisation_core::ron_config!("assets/config/attacks/boulder_throw.ron")
    }
}

// ---------------------------------------------------------------------------
// Animations
// ---------------------------------------------------------------------------

pub static BOULDER_ATTACK_ANIMATIONS: std::sync::LazyLock<HoveringAttackAnimations> =
    std::sync::LazyLock::new(|| {
        let hovering_canonical = AnimationData {
            finish_behavior: CxAnimationFinishBehavior::Loop,
            frames: 2,
            speed: 300,
            ..default()
        };

        let hit_canonical = AnimationData {
            finish_behavior: CxAnimationFinishBehavior::Mark,
            frames: 2,
            speed: 200,
            ..default()
        };

        let destroy_canonical = AnimationData {
            finish_behavior: CxAnimationFinishBehavior::Mark,
            frames: 2,
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
        let _config = BoulderThrowConfig::load();
    }

    #[test]
    fn values_match_original_constants() {
        let config = BoulderThrowConfig::load();
        assert!((config.depth_speed.get() - (-1.6)).abs() < f32::EPSILON);
        assert!((config.line_y_acceleration.get() - (-55.0)).abs() < f32::EPSILON);
        assert_eq!(config.damage, 45);
        assert!((config.randomness.get() - 35.0).abs() < f32::EPSILON);
    }
}
