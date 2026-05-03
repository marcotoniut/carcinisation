//! Serialized cutscene definitions: layers, steps, images, animations, music.
//!
//! Moved here from `apps/carcinisation/src/cutscene/data.rs` so the editor
//! (and other tools) can depend on these types without pulling in the
//! full application binary.

use crate::resources::CutsceneTimeDomain;
use bevy::prelude::*;
use carcinisation_base::layer::{Layer, MenuLayer};
use cween::animation::RotationKeyframe;
use cween::linear::components::{LinearTweenBundle, TargetingValueX, TargetingValueY};
use derive_new::new;
use serde::{Deserialize, Serialize};
use serde_with::{DurationSecondsWithFrac, serde_as};
use std::time::Duration;

// ---------------------------------------------------------------------------
// CutsceneData and related types
// ---------------------------------------------------------------------------

/// Controls which inputs can skip the cutscene.
#[derive(Clone, Debug, Default, Deserialize, Reflect, Serialize)]
pub enum CutsceneSkipMode {
    /// Only the Start button (default, existing behavior).
    #[default]
    StartOnly,
    /// Any gameplay key (A, B, Start, Select, arrows).
    AnyGameplayKey,
}

/// Shared rotation curve that all timeline-following elements reference.
#[derive(Clone, Debug, Deserialize, Reflect, Serialize)]
pub struct CutsceneTimelineConfig {
    pub rotation_keyframes: Vec<RotationKeyframe>,
    pub rotation_pivot: Vec2,
    pub rotation_position: IVec2,
}

/// Full-screen background rectangle spawned at act start.
#[derive(Clone, Debug, Deserialize, Reflect, Serialize)]
pub struct CutsceneBackgroundPrimitive {
    pub palette_index: u8,
    #[serde(default = "default_bg_layer")]
    pub layer: Layer,
}

fn default_bg_layer() -> Layer {
    Layer::Menu(MenuLayer::Background)
}

/// Movement description for animated elements.
#[derive(Clone, Copy, Debug, Default, Deserialize, Reflect, Serialize)]
pub struct TargetMovement {
    pub position: Vec2,
    #[serde(default)]
    pub speed: f32,
    #[serde(default)]
    pub acceleration: f32,
}

impl TargetMovement {
    /// Build tween bundles. Caller provides `game_base_speed`.
    #[must_use]
    pub fn make_bundles(
        self,
        coordinates: Vec2,
        game_base_speed: f32,
    ) -> (
        LinearTweenBundle<CutsceneTimeDomain, TargetingValueX>,
        LinearTweenBundle<CutsceneTimeDomain, TargetingValueY>,
    ) {
        let normalised_direction = (self.position - coordinates).normalize_or_zero();
        let velocity = normalised_direction * self.speed * game_base_speed;

        (
            LinearTweenBundle::<CutsceneTimeDomain, TargetingValueX>::new(
                coordinates.x,
                self.position.x,
                velocity.x,
            ),
            LinearTweenBundle::<CutsceneTimeDomain, TargetingValueY>::new(
                coordinates.y,
                self.position.y,
                velocity.y,
            ),
        )
    }
}

#[serde_as]
#[derive(new, Clone, Debug, Deserialize, Reflect, Serialize)]
pub struct CutsceneAnimationSpawn {
    pub image_path: String,
    pub frame_count: usize,
    #[serde_as(as = "DurationSecondsWithFrac")]
    pub duration: Duration,
    pub layer: Layer,
    #[new(default)]
    #[serde(default)]
    pub coordinates: Vec2,
    #[new(default)]
    #[serde(default)]
    pub tag_o: Option<String>,
    #[new(default)]
    #[serde(default)]
    pub target_movement_o: Option<TargetMovement>,
    #[new(default)]
    #[serde(default)]
    pub rotation_keyframes_o: Option<Vec<RotationKeyframe>>,
    #[new(default)]
    #[serde(default)]
    pub rotation_pivot_o: Option<Vec2>,
    #[new(default)]
    #[serde(default)]
    pub rotation_offset_deg: f32,
    #[new(default)]
    #[serde(default)]
    pub appear_ms_o: Option<u64>,
    #[new(default)]
    #[serde(default)]
    pub rotation_time_scale_o: Option<f32>,
}

#[derive(new, Clone, Debug, Component, Deserialize, Reflect, Serialize)]
pub struct CutsceneAnimationsSpawn {
    #[new(default)]
    pub spawns: Vec<CutsceneAnimationSpawn>,
}

#[derive(new, Clone, Debug, Component)]
pub struct CutsceneAwaitInput;

#[derive(new, Clone, Debug, Component)]
pub struct CutsceneElapse {
    pub duration: Duration,
    #[new(value = "false")]
    pub clear_graphics: bool,
}

#[derive(new, Clone, Debug, Deserialize, Reflect, Serialize)]
pub struct CutsceneImageSpawn {
    pub image_path: String,
    pub layer: Layer,
    #[new(default)]
    #[serde(default)]
    pub coordinates: Vec2,
    #[new(default)]
    #[serde(default)]
    pub tag_o: Option<String>,
    #[new(default)]
    #[serde(default)]
    pub rotation_keyframes_o: Option<Vec<RotationKeyframe>>,
    #[new(default)]
    #[serde(default)]
    pub rotation_pivot_o: Option<Vec2>,
    #[new(default)]
    #[serde(default)]
    pub rotation_offset_deg: f32,
    #[new(default)]
    #[serde(default)]
    pub appear_ms_o: Option<u64>,
    #[new(default)]
    #[serde(default)]
    pub rotation_time_scale_o: Option<f32>,
    #[new(default)]
    #[serde(default)]
    pub follow_rotation_tag_o: Option<String>,
}

#[derive(new, Clone, Component, Debug, Deserialize, Reflect, Serialize)]
pub struct CutsceneImagesSpawn {
    #[new(default)]
    pub spawns: Vec<CutsceneImageSpawn>,
}

impl CutsceneImagesSpawn {
    #[must_use]
    pub fn push_spawn(mut self, spawn: CutsceneImageSpawn) -> Self {
        self.spawns.push(spawn);
        self
    }

    #[must_use]
    pub fn with_spawns(mut self, spawns: Vec<CutsceneImageSpawn>) -> Self {
        self.spawns = spawns;
        self
    }
}

impl CutsceneImageSpawn {
    #[must_use]
    pub fn with_appear_ms(mut self, appear_ms_o: Option<u64>) -> Self {
        self.appear_ms_o = appear_ms_o;
        self
    }

    #[must_use]
    pub fn with_rotation_time_scale(mut self, time_scale_o: Option<f32>) -> Self {
        self.rotation_time_scale_o = time_scale_o;
        self
    }

    #[must_use]
    pub fn with_rotation_offset_deg(mut self, deg: f32) -> Self {
        self.rotation_offset_deg = deg;
        self
    }

    #[must_use]
    pub fn with_follow_rotation_tag(mut self, tag: impl Into<String>) -> Self {
        self.follow_rotation_tag_o = Some(tag.into());
        self
    }

    #[must_use]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tag_o = Some(tag.into());
        self
    }
}

#[derive(Clone, Debug, Deserialize, Reflect, Serialize)]
pub enum LetterboxMove {
    To(f32),
    ToAt(f32, f32),
    Hide,
    Show,
    Close,
    Open,
}

#[serde_as]
#[derive(new, Clone, Debug, Deserialize, Reflect, Serialize)]
pub struct CutsceneAct {
    #[new(default)]
    #[serde(default)]
    pub await_input: bool,
    #[new(default)]
    #[serde(default)]
    pub despawn_entities: Vec<String>,
    #[new(default)]
    #[serde(default)]
    #[serde_as(as = "DurationSecondsWithFrac")]
    pub elapse: Duration,
    #[new(default)]
    #[serde(default)]
    pub letterbox_move_o: Option<LetterboxMove>,
    #[new(default)]
    #[serde(default)]
    pub music_despawn_o: Option<CutsceneMusicDespawn>,
    #[new(default)]
    #[serde(default)]
    pub music_spawn_o: Option<CutsceneMusicSpawn>,
    #[new(default)]
    #[serde(default)]
    pub spawn_animations_o: Option<CutsceneAnimationsSpawn>,
    #[new(default)]
    #[serde(default)]
    pub spawn_images_o: Option<CutsceneImagesSpawn>,
    #[new(default)]
    #[serde(default)]
    pub transition_o: Option<CutsceneTransition>,
    #[new(default)]
    #[serde(default)]
    pub background_primitive_o: Option<CutsceneBackgroundPrimitive>,
}

impl CutsceneAct {
    #[must_use]
    pub fn move_letterbox(mut self, x: LetterboxMove) -> Self {
        self.letterbox_move_o = Some(x);
        self
    }

    #[must_use]
    pub fn spawn_animations(mut self, spawns: CutsceneAnimationsSpawn) -> Self {
        self.spawn_animations_o = Some(spawns);
        self
    }

    #[must_use]
    pub fn spawn_images(mut self, spawns: CutsceneImagesSpawn) -> Self {
        self.spawn_images_o = Some(spawns);
        self
    }

    #[must_use]
    pub fn spawn_music(mut self, spawn: CutsceneMusicSpawn) -> Self {
        self.music_spawn_o = Some(spawn);
        self
    }

    #[must_use]
    pub fn despawn_music(mut self) -> Self {
        self.music_despawn_o = Some(CutsceneMusicDespawn {});
        self
    }

    #[must_use]
    pub fn with_elapse(mut self, secs: f32) -> Self {
        self.elapse = Duration::from_secs_f32(secs);
        self
    }

    #[must_use]
    pub fn with_background_primitive(mut self, bg: CutsceneBackgroundPrimitive) -> Self {
        self.background_primitive_o = Some(bg);
        self
    }
}

#[derive(new, Clone, Component, Debug, Deserialize, Reflect, Serialize)]
pub struct CutsceneMusicSpawn {
    pub music_path: String,
}

#[derive(new, Clone, Component, Debug, Deserialize, Reflect, Serialize)]
pub struct CutsceneMusicDespawn {}

#[derive(new, Clone, Component, Debug, Deserialize, Reflect, Serialize)]
pub struct CutsceneSpriteSpawn {
    pub image_path: String,
    pub coordinates: Vec2,
    #[new(default)]
    pub tag_o: Option<String>,
}

#[derive(new, Clone, Component, Debug, Deserialize, Reflect, Serialize)]
pub struct CutsceneTransition {
    pub request: TransitionRequest,
}

#[derive(Clone, Debug, Deserialize, Reflect, Serialize)]
pub enum TransitionRequest {
    Venetian,
}

#[derive(new, Asset, Clone, Debug, Deserialize, Reflect, Resource, Serialize)]
pub struct CutsceneData {
    pub name: String,
    #[new(default)]
    pub steps: Vec<CutsceneAct>,
    #[new(default)]
    #[serde(default)]
    pub skip_mode: CutsceneSkipMode,
    #[new(value = "true")]
    #[serde(default = "default_true")]
    pub respect_skip_cutscenes: bool,
    #[new(default)]
    #[serde(default)]
    pub timeline_config_o: Option<CutsceneTimelineConfig>,
}

fn default_true() -> bool {
    true
}

impl CutsceneData {
    #[must_use]
    pub fn set_steps(mut self, steps: Vec<CutsceneAct>) -> Self {
        self.steps = steps;
        self
    }

    #[must_use]
    pub fn with_skip_mode(mut self, mode: CutsceneSkipMode) -> Self {
        self.skip_mode = mode;
        self
    }

    #[must_use]
    pub fn with_respect_skip_cutscenes(mut self, respect: bool) -> Self {
        self.respect_skip_cutscenes = respect;
        self
    }

    #[must_use]
    pub fn with_timeline_config(mut self, config: CutsceneTimelineConfig) -> Self {
        self.timeline_config_o = Some(config);
        self
    }
}

// ---------------------------------------------------------------------------
// LetterboxMoveEvent (runtime event, kept here for convenience)
// ---------------------------------------------------------------------------

#[derive(new, Clone, Debug, Deserialize, Event, Serialize)]
pub struct LetterboxMoveEvent {
    pub speed: f32,
    pub target: f32,
}

impl LetterboxMoveEvent {
    #[must_use]
    pub fn open() -> Self {
        Self::new(LETTERBOX_NORMAL_SPEED, LETTERBOX_HEIGHT as f32)
    }

    #[must_use]
    pub fn close() -> Self {
        Self::new(LETTERBOX_NORMAL_SPEED, 0.0)
    }

    #[must_use]
    pub fn show() -> Self {
        Self::new(LETTERBOX_INSTANT_SPEED, LETTERBOX_HEIGHT as f32)
    }

    #[must_use]
    pub fn hide() -> Self {
        Self::new(LETTERBOX_INSTANT_SPEED, 0.0)
    }

    #[must_use]
    pub fn move_to(target: f32) -> Self {
        Self::new(LETTERBOX_NORMAL_SPEED, target)
    }

    #[must_use]
    pub fn move_to_at(target: f32, speed: f32) -> Self {
        Self::new(speed, target)
    }
}

impl From<LetterboxMove> for LetterboxMoveEvent {
    fn from(x: LetterboxMove) -> Self {
        match x {
            LetterboxMove::To(target) => LetterboxMoveEvent::move_to(target),
            LetterboxMove::ToAt(target, speed) => LetterboxMoveEvent::move_to_at(target, speed),
            LetterboxMove::Hide => LetterboxMoveEvent::hide(),
            LetterboxMove::Show => LetterboxMoveEvent::show(),
            LetterboxMove::Close => LetterboxMoveEvent::close(),
            LetterboxMove::Open => LetterboxMoveEvent::open(),
        }
    }
}

// Constants for letterbox. Defined here so both app and editor can use them.
pub const LETTERBOX_NORMAL_SPEED: f32 = 10.;
pub const LETTERBOX_INSTANT_SPEED: f32 = f32::MAX;
pub const LETTERBOX_HEIGHT: u32 = 20;
