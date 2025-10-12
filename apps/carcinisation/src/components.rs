use std::time::Duration;

use assert_assets_path::assert_assets_path;
use bevy::{audio::Volume, prelude::*};
use seldom_pixel::filter::PxFilterAsset;

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
    pub fn get_filter_path(&self) -> &'static str {
        match self {
            GBColor::Black => {
                assert_assets_path!("filter/color0.px_filter.png")
            }
            GBColor::DarkGray => {
                assert_assets_path!("filter/color1.px_filter.png")
            }
            GBColor::LightGray => {
                assert_assets_path!("filter/color2.px_filter.png")
            }
            GBColor::White => {
                assert_assets_path!("filter/color3.px_filter.png")
            }
        }
    }
}

pub trait PxSpriteColorLoader {
    /// Runs `f` on `self`
    fn load_color(&self, color: GBColor) -> Handle<PxFilterAsset>;
}

impl PxSpriteColorLoader for AssetServer {
    fn load_color(&self, color: GBColor) -> Handle<PxFilterAsset> {
        self.load(color.get_filter_path())
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

#[derive(Component)]
pub struct DelayedDespawnOnPxAnimationFinished(pub Duration);

impl DelayedDespawnOnPxAnimationFinished {
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
            master: Volume::new(0.8),
            music: Volume::new(0.06),
            sfx: Volume::new(0.08),
        }
    }
}
