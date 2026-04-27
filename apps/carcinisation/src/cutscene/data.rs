//! Serialized cutscene definitions: layers, steps, images, animations, music.

use super::resources::CutsceneTimeDomain;
use crate::{
    data::keyframe::RotationKeyframe, layer::Layer, letterbox::messages::LetterboxMove,
    stage::data::GAME_BASE_SPEED, transitions::data::TransitionRequest,
};
use bevy::prelude::*;
use cween::linear::components::{LinearTweenBundle, TargetingValueX, TargetingValueY};
use derive_new::new;
use serde::{Deserialize, Serialize};
use serde_with::{DurationSecondsWithFrac, serde_as};
use std::time::Duration;

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
    Layer::UIBackground
}

#[derive(Clone, Debug, Deserialize, PartialEq, PartialOrd, Eq, Ord, Reflect, Serialize)]
/// Layer stack used while rendering cinematic sequences (the later variants render on top).
pub enum CutsceneLayer {
    /// Parallax sky/props furthest from the camera (multiple slots for effects).
    Background(u8),
    /// Hero/enemy sprites and props that sit behind the letterbox/frame.
    Middle(u8),
    /// Black bars and frame dressing.
    Letterbox,
    /// Particles/effects above the letterbox but below UI text.
    Foreground(u8),
    /// Panel container for dialogue.
    Textbox,
    /// Primary dialogue text.
    Text,
    /// Captions/emphasis sitting above the main dialogue.
    Overtext(u8),
}

impl Default for CutsceneLayer {
    fn default() -> Self {
        CutsceneLayer::Background(0)
    }
}

#[serde_as]
#[derive(Clone, Copy, Debug, Default, Deserialize, Reflect, Serialize)]
pub struct TargetMovement {
    pub position: Vec2,
    #[serde(default)]
    pub speed: f32,
    #[serde(default)]
    pub acceleration: f32,
}

impl TargetMovement {
    #[must_use]
    pub fn make_bundles(
        self,
        coordinates: Vec2,
    ) -> (
        LinearTweenBundle<CutsceneTimeDomain, TargetingValueX>,
        LinearTweenBundle<CutsceneTimeDomain, TargetingValueY>,
    ) {
        let normalised_direction = (self.position - coordinates).normalize_or_zero();
        let velocity = normalised_direction * self.speed * GAME_BASE_SPEED;

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
    /// Rotation keyframes evaluated against `CutsceneTimeDomain`.
    #[new(default)]
    #[serde(default)]
    pub rotation_keyframes_o: Option<Vec<RotationKeyframe>>,
    /// Anchor pivot for rotation (normalised 0–1). Defaults to BottomLeft.
    #[new(default)]
    #[serde(default)]
    pub rotation_pivot_o: Option<Vec2>,
    /// Static angle offset in degrees (compensate for pre-rotated art).
    #[new(default)]
    #[serde(default)]
    pub rotation_offset_deg: f32,
    /// Delay before this element appears (ms in cutscene time).
    /// When set, the element starts hidden and is revealed at this time.
    #[new(default)]
    #[serde(default)]
    pub appear_ms_o: Option<u64>,
    /// Follow the shared `CutsceneTimelineConfig` rotation curve at this
    /// time scale (1.0 = match, <1.0 = lag behind). Mutually exclusive with
    /// `rotation_keyframes_o` — if both are set, keyframes take priority.
    #[new(default)]
    #[serde(default)]
    pub rotation_time_scale_o: Option<f32>,
}

#[derive(new, Clone, Component, Debug, Deserialize, Reflect, Serialize)]
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
    /// Rotation keyframes evaluated against `CutsceneTimeDomain`.
    #[new(default)]
    #[serde(default)]
    pub rotation_keyframes_o: Option<Vec<RotationKeyframe>>,
    /// Anchor pivot for rotation (normalised 0–1). Defaults to BottomLeft.
    #[new(default)]
    #[serde(default)]
    pub rotation_pivot_o: Option<Vec2>,
    /// Static angle offset in degrees.
    #[new(default)]
    #[serde(default)]
    pub rotation_offset_deg: f32,
    /// Delay before this element appears (ms in cutscene time).
    #[new(default)]
    #[serde(default)]
    pub appear_ms_o: Option<u64>,
    /// Follow the shared timeline rotation curve at this time scale.
    #[new(default)]
    #[serde(default)]
    pub rotation_time_scale_o: Option<f32>,
    /// Tag of a leader track whose rotation this element follows.
    /// When set, `rotation_keyframes_o` is interpreted as *relative offset*
    /// keyframes added to the leader's rotation.
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
    // TODO fade_in
}

#[derive(new, Clone, Component, Debug, Deserialize, Reflect, Serialize)]
pub struct CutsceneMusicDespawn {
    // TODO fade_out
}

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

#[derive(new, Asset, Clone, Debug, Deserialize, Reflect, Resource, Serialize)]
pub struct CutsceneData {
    pub name: String,
    #[new(default)]
    pub steps: Vec<CutsceneAct>,
    /// Controls which inputs skip the cutscene.
    #[new(default)]
    #[serde(default)]
    pub skip_mode: CutsceneSkipMode,
    /// When false, `CARCINISATION_SKIP_CUTSCENES` does not auto-skip this cutscene.
    #[new(value = "true")]
    #[serde(default = "default_true")]
    pub respect_skip_cutscenes: bool,
    /// Shared rotation timeline that elements can follow via `rotation_time_scale_o`.
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
