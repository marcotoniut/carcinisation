use crate::{data::AnimationData, stage::attack::data::HoveringAttackAnimations};
use bevy::prelude::*;
use carapace::prelude::CxAnimationFinishBehavior;
use serde::Deserialize;

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

const CONFIG_PATH: &str = "assets/config/attacks/boulder_throw.ron";
const EMBEDDED_CONFIG: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/config/attacks/boulder_throw.ron"
));

/// Tuning parameters for the boulder throw projectile.
///
/// Loaded from `assets/config/attacks/boulder_throw.ron`.  See
/// [`super::blood_shot::BloodShotConfig`] for the loading model.
#[derive(Clone, Debug, Deserialize, Resource, Reflect)]
#[reflect(Resource)]
pub struct BoulderThrowConfig {
    pub depth_speed: f32,
    pub line_y_acceleration: f32,
    pub damage: u32,
    pub randomness: f32,
}

impl BoulderThrowConfig {
    #[must_use]
    pub fn load() -> Self {
        #[cfg(not(target_family = "wasm"))]
        if let Ok(body) = std::fs::read_to_string(CONFIG_PATH) {
            return Self::parse_and_validate(&body, CONFIG_PATH);
        }
        Self::parse_and_validate(EMBEDDED_CONFIG, "embedded boulder_throw.ron")
    }

    fn parse_and_validate(ron_str: &str, source: &str) -> Self {
        let config: Self = ron::from_str(ron_str).unwrap_or_else(|e| {
            panic!("{source}: failed to parse BoulderThrowConfig: {e}");
        });
        config.validate(source);
        config
    }

    fn validate(&self, source: &str) {
        assert!(
            self.depth_speed.is_finite(),
            "{source}: depth_speed must be finite",
        );
        assert!(
            self.line_y_acceleration.is_finite(),
            "{source}: line_y_acceleration must be finite",
        );
        assert!(
            self.randomness.is_finite() && self.randomness >= 0.0,
            "{source}: randomness must be finite and non-negative, got {}",
            self.randomness,
        );
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
        let config: BoulderThrowConfig = ron::from_str(EMBEDDED_CONFIG)
            .expect("embedded boulder_throw.ron must parse into BoulderThrowConfig");
        config.validate("embedded boulder_throw.ron");
    }

    #[test]
    fn values_match_original_constants() {
        let config = BoulderThrowConfig::load();
        assert!((config.depth_speed - (-1.6)).abs() < f32::EPSILON);
        assert!((config.line_y_acceleration - (-55.0)).abs() < f32::EPSILON);
        assert_eq!(config.damage, 45);
        assert!((config.randomness - 35.0).abs() < f32::EPSILON);
    }
}
