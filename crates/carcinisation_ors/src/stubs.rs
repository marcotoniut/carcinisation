//! Stub types and functions for dependencies that remain in the app crate.
//!
//! Each item below is a placeholder for a type or function that lives in
//! `apps/carcinisation` and has not yet been extracted into a shared crate.
//! They exist solely to let this crate compile.  The app crate is expected
//! to wire real implementations via re-exports or feature flags.

use bevy::prelude::*;

// ---------------------------------------------------------------------------
// Game / score — resolved: now in carcinisation_base::game
// ---------------------------------------------------------------------------

pub use carcinisation_base::game::{CameraPos, GameProgressState, Lives, Score};

// TODO(ors-extract): resolve this dependency — lives in `game::resources`
/// Tracks which game step is currently active.
#[derive(Resource, Default, Clone, Copy)]
pub struct GameProgress {
    pub index: usize,
}

// TODO(ors-extract): resolve this dependency — lives in `game::data`
pub const STARTING_LIVES: u8 = 3;

// TODO(ors-extract): resolve this dependency — lives in `game::data`
pub const DEATH_SCORE_PENALTY: i32 = 150;

// TODO(ors-extract): resolve this dependency — lives in `game::messages`
#[derive(Clone, Event, Message)]
pub struct GameOverEvent {
    pub score: u32,
}

// TODO(ors-extract): resolve this dependency — lives in `game`
#[derive(activable::Activable)]
pub struct GamePlugin;

// ---------------------------------------------------------------------------
// Debug
// ---------------------------------------------------------------------------

// TODO(ors-extract): resolve this dependency — lives in `debug`
#[derive(Resource, Default)]
pub struct DebugGodMode {
    pub enabled: bool,
}

impl DebugGodMode {
    #[must_use]
    pub const fn new(enabled: bool) -> Self {
        Self { enabled }
    }
}

// TODO(ors-extract): resolve this dependency — lives in `debug::plugin`
pub fn debug_print_startup(module: &str) {
    #[cfg(debug_assertions)]
    bevy::log::info!("[DEBUG] {module} startup");
}

// TODO(ors-extract): resolve this dependency — lives in `debug::plugin`
pub fn debug_print_shutdown(module: &str) {
    #[cfg(debug_assertions)]
    bevy::log::info!("[DEBUG] {module} shutdown");
}

// ---------------------------------------------------------------------------
// Splash
// ---------------------------------------------------------------------------

// TODO(ors-extract): resolve this dependency — lives in `splash::components`
#[derive(Resource)]
pub struct SplashActive;

// ---------------------------------------------------------------------------
// Globals (app-specific)
// ---------------------------------------------------------------------------

// TODO(ors-extract): resolve this dependency — lives in `globals`
pub const ASSETS_PATH: &str = "../../assets";

// TODO(ors-extract): resolve this dependency — lives in `globals`
pub const PATH_SPRITES_ENEMIES: &str = assert_assets_path::assert_assets_path!("sprites/enemies/");

// TODO(ors-extract): resolve this dependency — lives in `globals`
pub const PATH_SPRITES_OBJECTS: &str = assert_assets_path::assert_assets_path!("sprites/objects/");

// TODO(ors-extract): resolve this dependency — lives in `globals`
pub const DEBUG_STAGESTEP: bool = false;

// TODO(ors-extract): resolve this dependency — lives in `globals`
pub const DEFAULT_CROSSHAIR_INDEX: u8 = 1;

// TODO(ors-extract): resolve this dependency — lives in `globals`
#[must_use]
pub fn load_inverted_typeface(
    assets: &crate::assets::CxAssets<'_, '_, carapace::prelude::CxTypeface>,
) -> Handle<carapace::prelude::CxTypeface> {
    const TYPEFACE_INVERTED_PATH: &str =
        assert_assets_path::assert_assets_path!("typeface/pixeboy-inverted.px_typeface.png");
    const TYPEFACE_CHARACTERS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()-_=+[{]}\\|;:'\",<.>/?";
    assets.load(TYPEFACE_INVERTED_PATH, TYPEFACE_CHARACTERS, [(' ', 4)])
}

// ---------------------------------------------------------------------------
// Cutscene
// ---------------------------------------------------------------------------

// TODO(ors-extract): resolve this dependency — lives in `cutscene::data`
#[derive(Clone, Component, Debug, serde::Deserialize, Reflect, serde::Serialize)]
pub struct CutsceneAnimationsSpawn {
    #[serde(default)]
    pub spawns: Vec<CutsceneAnimationSpawn>,
}

// TODO(ors-extract): resolve this dependency — lives in `cutscene::data`
#[derive(Clone, Debug, serde::Deserialize, Reflect, serde::Serialize)]
pub struct CutsceneAnimationSpawn {
    pub image_path: String,
    pub frame_count: usize,
    #[serde(with = "crate::stubs::duration_secs_frac")]
    pub duration: std::time::Duration,
    pub layer: carcinisation_base::layer::Layer,
    #[serde(default)]
    pub coordinates: Vec2,
    #[serde(default)]
    pub tag_o: Option<String>,
    #[serde(default)]
    pub rotation_time_scale_o: Option<f32>,
}

// Minimal (de)serialization for Duration as f64 seconds,
// standing in for `serde_with::DurationSecondsWithFrac`.
mod duration_secs_frac {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S: Serializer>(d: &Duration, s: S) -> Result<S::Ok, S::Error> {
        d.as_secs_f64().serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Duration, D::Error> {
        let secs = f64::deserialize(d)?;
        Ok(Duration::from_secs_f64(secs))
    }
}

// ---------------------------------------------------------------------------
// Camera
// ---------------------------------------------------------------------------

// CameraPos: resolved — now in carcinisation_base::game

// ---------------------------------------------------------------------------
// Systems / spawn
// ---------------------------------------------------------------------------

// TODO(ors-extract): resolve this dependency — lives in `systems::spawn`
#[must_use]
pub fn make_music_bundle(
    asset_server: &Res<AssetServer>,
    volume_settings: &Res<carcinisation_core::components::VolumeSettings>,
    music_path: String,
    mode: bevy::audio::PlaybackMode,
) -> (
    bevy::audio::AudioPlayer,
    bevy::audio::PlaybackSettings,
    carcinisation_core::components::AudioSystemBundle,
    carcinisation_core::components::Music,
) {
    let source = asset_server.load(music_path);
    (
        bevy::audio::AudioPlayer::new(source),
        bevy::audio::PlaybackSettings {
            mode,
            volume: volume_settings.music,
            ..Default::default()
        },
        carcinisation_core::components::AudioSystemBundle {
            system_type: carcinisation_core::components::AudioSystemType::MUSIC,
        },
        carcinisation_core::components::Music,
    )
}

// ---------------------------------------------------------------------------
// Movement
// ---------------------------------------------------------------------------

// TODO(ors-extract): resolve this dependency — lives in `systems::movement`
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct PositionSyncSystems;

// ---------------------------------------------------------------------------
// Transitions
// ---------------------------------------------------------------------------

// TODO(ors-extract): resolve this dependency — lives in `transitions::data`
#[derive(Clone, Debug, Default, serde::Deserialize, Reflect, serde::Serialize)]
#[reflect(Default)]
pub enum TransitionRequest {
    #[default]
    Venetian,
}

// TODO(ors-extract): resolve this dependency — lives in `transitions`
pub fn trigger_transition(_commands: &mut Commands, _request: &TransitionRequest) {
    // No-op stub — the real implementation triggers a transition animation.
}

// ---------------------------------------------------------------------------
// Main menu
// ---------------------------------------------------------------------------

// TODO(ors-extract): resolve this dependency — lives in `main_menu`
#[derive(activable::Activable)]
pub struct MainMenuPlugin;

// ---------------------------------------------------------------------------
// Systems (delay despawn)
// ---------------------------------------------------------------------------

// delay_despawn / check_despawn_after_delay: resolved — now in carcinisation_core::globals
pub use carcinisation_core::globals::{check_despawn_after_delay, delay_despawn};
