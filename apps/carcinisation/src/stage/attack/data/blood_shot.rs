use crate::{data::AnimationData, stage::attack::data::HoveringAttackAnimations};
use bevy::prelude::*;
use carapace::prelude::PxAnimationFinishBehavior;
use serde::Deserialize;
use std::time::Duration;

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

const CONFIG_PATH: &str = "assets/config/attacks/blood_shot.ron";
const EMBEDDED_CONFIG: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/config/attacks/blood_shot.ron"
));

/// Tuning parameters for the blood shot projectile.
///
/// Loaded from `assets/config/attacks/blood_shot.ron`.  The checked-in RON
/// is embedded at compile time via `include_str!` (canonical default, used
/// on WASM).  On native, a filesystem override is loaded if present —
/// malformed overrides cause a panic with a clear error.
#[derive(Clone, Debug, Deserialize, Resource, Reflect)]
#[reflect(Resource)]
pub struct BloodShotConfig {
    pub depth_speed: f32,
    pub line_speed: f32,
    pub damage: u32,
    pub randomness: f32,
    pub startup_hold_ms: u64,
}

impl BloodShotConfig {
    /// Load config: native tries the filesystem first, WASM uses embedded.
    ///
    /// # Panics
    ///
    /// Panics if a filesystem override exists but fails to parse or validate.
    #[must_use]
    pub fn load() -> Self {
        #[cfg(not(target_family = "wasm"))]
        if let Ok(body) = std::fs::read_to_string(CONFIG_PATH) {
            return Self::parse_and_validate(&body, CONFIG_PATH);
        }

        Self::parse_and_validate(EMBEDDED_CONFIG, "embedded blood_shot.ron")
    }

    fn parse_and_validate(ron_str: &str, source: &str) -> Self {
        let config: Self = ron::from_str(ron_str).unwrap_or_else(|e| {
            panic!("{source}: failed to parse BloodShotConfig: {e}");
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

    /// Startup hold as a [`Duration`].
    #[must_use]
    pub fn startup_hold(&self) -> Duration {
        Duration::from_millis(self.startup_hold_ms)
    }
}

// ---------------------------------------------------------------------------
// Animations (not config-driven — structural, rarely changes)
// ---------------------------------------------------------------------------

pub static BLOOD_ATTACK_ANIMATIONS: std::sync::LazyLock<HoveringAttackAnimations> =
    std::sync::LazyLock::new(|| {
        let hovering_canonical = AnimationData {
            finish_behavior: PxAnimationFinishBehavior::Loop,
            frames: 4,
            speed: 700,
            ..default()
        };

        let hit_canonical = AnimationData {
            finish_behavior: PxAnimationFinishBehavior::Mark,
            frames: 1,
            speed: 300,
            ..default()
        };

        let destroy_canonical = AnimationData {
            finish_behavior: PxAnimationFinishBehavior::Mark,
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
        let config: BloodShotConfig = ron::from_str(EMBEDDED_CONFIG)
            .expect("embedded blood_shot.ron must parse into BloodShotConfig");
        config.validate("embedded blood_shot.ron");
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
        assert!((config.depth_speed - (-5.0)).abs() < f32::EPSILON);
        assert!((config.line_speed - 55.0).abs() < f32::EPSILON);
        assert_eq!(config.damage, 20);
        assert!((config.randomness - 20.0).abs() < f32::EPSILON);
        assert_eq!(config.startup_hold_ms, 60);
    }
}
