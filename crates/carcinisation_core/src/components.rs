use std::time::Duration;

use assert_assets_path::assert_assets_path;
use bevy::{audio::Volume, prelude::*};

#[allow(clippy::upper_case_acronyms)]
#[derive(Component)]
pub enum AudioSystemType {
    SFX,
    MUSIC,
}

#[derive(Bundle)]
pub struct AudioSystemBundle {
    pub system_type: AudioSystemType,
}

#[derive(Clone, Component, Copy, Default, Debug, Reflect, PartialEq, Eq, Hash)]
pub enum GBColor {
    #[default]
    Black,
    DarkGray,
    LightGray,
    White,
}

impl GBColor {
    #[must_use]
    pub const fn get_filter_path(&self) -> &'static str {
        match self {
            Self::Black => {
                assert_assets_path!("filter/color0.px_filter.png")
            }
            Self::DarkGray => {
                assert_assets_path!("filter/color1.px_filter.png")
            }
            Self::LightGray => {
                assert_assets_path!("filter/color2.px_filter.png")
            }
            Self::White => {
                assert_assets_path!("filter/color3.px_filter.png")
            }
        }
    }
}

#[derive(Component)]
pub struct Tag(pub String);

#[derive(Component)]
pub struct Cleared;

#[derive(Component)]
pub struct CutsceneElapsedStarted(pub Duration);

#[derive(Component)]
pub struct DespawnMark;

#[derive(Component)]
pub struct Music;

/// Marker resource indicating the splash screen is active.
#[derive(Resource)]
pub struct SplashActive;

#[derive(Component)]
pub struct DelayedDespawnOnCxAnimationFinished(pub Duration);

impl DelayedDespawnOnCxAnimationFinished {
    #[must_use]
    pub fn from_secs_f32(secs: f32) -> Self {
        Self(Duration::from_secs_f32(secs))
    }
}

#[derive(Component)]
pub struct DespawnAfterDelay {
    pub elapsed: Duration,
    pub duration: Duration,
}

// TODO could probably split into different resources
#[derive(Resource, Clone, Copy, Debug)]
pub struct VolumeSettings {
    pub master: Volume,
    pub music: Volume,
    pub sfx: Volume,
}

impl Default for VolumeSettings {
    fn default() -> Self {
        Self {
            master: Volume::Linear(0.8),
            music: Volume::Linear(0.06),
            sfx: Volume::Linear(0.08),
        }
    }
}

impl VolumeSettings {
    #[must_use]
    pub fn with_master_level(self, master_level: f32) -> Self {
        Self {
            master: Volume::Linear(master_level),
            music: scaled_volume(self.music, master_level),
            sfx: scaled_volume(self.sfx, master_level),
        }
    }
}

fn scaled_volume(volume: Volume, factor: f32) -> Volume {
    match volume {
        Volume::Linear(level) => Volume::Linear(level * factor),
        Volume::Decibels(level) => Volume::Decibels(level * factor),
    }
}

/// Build a music playback bundle with volume settings.
#[must_use]
pub fn make_music_bundle(
    asset_server: &Res<AssetServer>,
    volume_settings: &Res<VolumeSettings>,
    music_path: String,
    mode: bevy::audio::PlaybackMode,
) -> (
    bevy::audio::AudioPlayer,
    bevy::audio::PlaybackSettings,
    AudioSystemBundle,
    Music,
) {
    let source = asset_server.load(music_path);
    (
        bevy::audio::AudioPlayer::new(source),
        bevy::audio::PlaybackSettings {
            mode,
            volume: volume_settings.music,
            ..Default::default()
        },
        AudioSystemBundle {
            system_type: AudioSystemType::MUSIC,
        },
        Music,
    )
}
