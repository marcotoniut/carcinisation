//! Data-driven player tuning loaded from `assets/config/player.ron`.

use bevy::prelude::*;
use carapace::constrained::PositiveFiniteF32;
use carcinisation_core::ron_config;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, Resource, Reflect)]
#[reflect(Resource)]
pub struct PlayerConfig {
    /// Base movement speed in pixels per second.
    pub base_speed: PositiveFiniteF32,
    /// Movement speed multiplier when the slow modifier (B held) is active.
    pub slow_modifier: f32,
}

impl PlayerConfig {
    #[must_use]
    pub fn load() -> Self {
        let config: Self = ron_config!("assets/config/player.ron");
        config.validate();
        config
    }

    fn validate(&self) {
        assert!(
            self.slow_modifier > 0.0 && self.slow_modifier <= 1.0 && self.slow_modifier.is_finite(),
            "PlayerConfig: slow_modifier must be in (0.0, 1.0], got {}",
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
        assert!(config.base_speed.get() > 0.0);
        assert!(config.slow_modifier > 0.0 && config.slow_modifier <= 1.0);
    }
}
