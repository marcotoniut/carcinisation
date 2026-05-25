//! Splash RON config — per-track animation data, loaded and mapped to
//! `CutsceneData` at startup.

use cween::animation::RotationKeyframe;
use serde::Deserialize;
use std::num::NonZeroU64;

/// One animation track — a sprite with optional timing, pivot, and keyframes.
#[derive(Clone, Debug, Deserialize)]
pub struct SplashTrack {
    pub asset: String,
    #[serde(default)]
    pub pivot: Option<(f32, f32)>,
    #[serde(default)]
    pub position: Option<(i32, i32)>,
    #[serde(default)]
    pub appear_ms: Option<u64>,
    /// Rotation keyframes. Interpreted as absolute unless `follow_rotation_tag`
    /// is set, in which case they're relative offsets from the leader's rotation.
    #[serde(default)]
    pub rotation: Option<Vec<RotationKeyframe>>,
    /// Tag name for this track (so followers can reference it).
    #[serde(default)]
    pub tag: Option<String>,
    /// Tag of a leader track. This track's rotation = leader's rotation +
    /// these keyframes as relative offset.
    #[serde(default)]
    pub follow_rotation_tag: Option<String>,
}

/// Top-level splash config loaded from RON.
#[derive(Clone, Debug, Deserialize)]
pub struct SplashConfig {
    pub total_duration_ms: NonZeroU64,
    pub slowdown: u32,
    pub bg_palette_index: u8,
    pub tracks: Vec<SplashTrack>,
}

impl SplashConfig {
    #[must_use]
    pub fn load() -> Self {
        let config: Self = carcinisation_core::ron_config!("assets/splash/bevy.ron");
        config.validate();
        config
    }

    fn validate(&self) {
        assert!(
            !self.tracks.is_empty(),
            "SplashConfig: tracks must not be empty",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splash_config_loads() {
        let _ = SplashConfig::load();
    }
}
