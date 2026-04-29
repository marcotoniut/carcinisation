use crate::{data::AnimationData, stage::attack::data::HoveringAttackAnimations};
use bevy::prelude::*;
use carapace::prelude::CxAnimationFinishBehavior;
use serde::Deserialize;
use std::time::Duration;

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

const CONFIG_PATH: &str = "assets/config/attacks/spider_shot.ron";
const EMBEDDED_CONFIG: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/config/attacks/spider_shot.ron"
));

/// Tuning parameters for the spider shot projectile.
///
/// Loaded from `assets/config/attacks/spider_shot.ron`.  See
/// [`super::blood_shot::BloodShotConfig`] for the loading model.
#[derive(Clone, Debug, Deserialize, Resource, Reflect)]
#[reflect(Resource)]
pub struct SpiderShotConfig {
    pub depth_speed: f32,
    /// Retained for future use / tuning reference. The current spawn uses
    /// time-matched velocity (displacement / depth_time) instead of a fixed
    /// lateral speed, so this field is not read at runtime.
    pub line_speed: f32,
    pub damage: u32,
    pub randomness: f32,
    pub startup_hold_ms: u64,
}

impl SpiderShotConfig {
    /// How long the projectile stays at its spawn point before beginning travel.
    #[must_use]
    pub fn startup_hold(&self) -> Duration {
        Duration::from_millis(self.startup_hold_ms)
    }
}

impl SpiderShotConfig {
    #[must_use]
    pub fn load() -> Self {
        #[cfg(not(target_family = "wasm"))]
        if let Ok(body) = std::fs::read_to_string(CONFIG_PATH) {
            return Self::parse_and_validate(&body, CONFIG_PATH);
        }
        Self::parse_and_validate(EMBEDDED_CONFIG, "embedded spider_shot.ron")
    }

    fn parse_and_validate(ron_str: &str, source: &str) -> Self {
        let config: Self = ron::from_str(ron_str).unwrap_or_else(|e| {
            panic!("{source}: failed to parse SpiderShotConfig: {e}");
        });
        config.validate(source);
        config
    }

    fn validate(&self, source: &str) {
        assert!(
            self.line_speed > 0.0,
            "{source}: line_speed must be positive, got {}",
            self.line_speed,
        );
        assert!(
            self.line_speed.is_finite(),
            "{source}: line_speed must be finite",
        );
        assert!(
            self.depth_speed.is_finite(),
            "{source}: depth_speed must be finite",
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
        let config: SpiderShotConfig = ron::from_str(EMBEDDED_CONFIG)
            .expect("embedded spider_shot.ron must parse into SpiderShotConfig");
        config.validate("embedded spider_shot.ron");
    }

    #[test]
    fn values_match_original_constants() {
        let config = SpiderShotConfig::load();
        assert!((config.depth_speed - (-4.0)).abs() < f32::EPSILON);
        assert!((config.line_speed - 45.0).abs() < f32::EPSILON);
        assert_eq!(config.damage, 5);
        assert!((config.randomness - 15.0).abs() < f32::EPSILON);
        assert_eq!(config.startup_hold_ms, 80);
    }
}
