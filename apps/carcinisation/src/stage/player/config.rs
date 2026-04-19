//! Data-driven player tuning loaded from `assets/config/player.ron`.

use bevy::prelude::*;
use serde::Deserialize;

const CONFIG_PATH: &str = "assets/config/player.ron";
const EMBEDDED_CONFIG: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/config/player.ron"
));

#[derive(Clone, Debug, Deserialize, Resource, Reflect)]
#[reflect(Resource)]
pub struct PlayerConfig {
    /// Base movement speed in pixels per second.
    pub base_speed: f32,
    /// Movement speed multiplier when the slow modifier (B held) is active.
    pub slow_modifier: f32,
}

impl PlayerConfig {
    pub fn load() -> Self {
        #[cfg(not(target_family = "wasm"))]
        if let Ok(body) = std::fs::read_to_string(CONFIG_PATH) {
            return Self::parse_and_validate(&body, CONFIG_PATH);
        }

        Self::parse_and_validate(EMBEDDED_CONFIG, "embedded player.ron")
    }

    fn parse_and_validate(ron_str: &str, source: &str) -> Self {
        let config: Self = ron::from_str(ron_str).unwrap_or_else(|e| {
            panic!("{source}: failed to parse PlayerConfig: {e}");
        });
        config.validate(source);
        config
    }

    fn validate(&self, source: &str) {
        assert!(
            self.base_speed > 0.0 && self.base_speed.is_finite(),
            "{source}: base_speed must be positive and finite, got {}",
            self.base_speed
        );
        assert!(
            self.slow_modifier > 0.0 && self.slow_modifier <= 1.0 && self.slow_modifier.is_finite(),
            "{source}: slow_modifier must be in (0.0, 1.0], got {}",
            self.slow_modifier
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_config_parses_and_validates() {
        let config = PlayerConfig::load();
        assert!(config.base_speed > 0.0);
        assert!(config.slow_modifier > 0.0 && config.slow_modifier <= 1.0);
    }
}
