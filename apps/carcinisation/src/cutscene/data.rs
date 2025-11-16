//! Serialized cutscene definitions: layers, steps, images, animations, music.

use super::resources::CutsceneTimeDomain;
use crate::{
    layer::Layer,
    letterbox::events::LetterboxMove,
    plugins::movement::linear::components::{
        LinearMovementBundle, TargetingPositionX, TargetingPositionY,
    },
    stage::data::GAME_BASE_SPEED,
    transitions::data::TransitionRequest,
};
use bevy::prelude::*;
use derive_new::new;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationSecondsWithFrac};
use std::time::Duration;

#[cfg(feature = "derive-ts")]
use ts_rs::TS;

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
#[cfg_attr(feature = "derive-ts", derive(TS))]
#[cfg_attr(feature = "derive-ts", ts(export))]
#[derive(Clone, Copy, Debug, Default, Deserialize, Reflect, Serialize)]
pub struct TargetMovement {
    #[cfg_attr(feature = "derive-ts", ts(type = "[number, number]"))]
    pub position: Vec2,
    #[serde(default)]
    pub speed: f32,
    #[serde(default)]
    pub acceleration: f32,
}

impl TargetMovement {
    pub fn make_bundles(
        self,
        coordinates: Vec2,
    ) -> (
        LinearMovementBundle<CutsceneTimeDomain, TargetingPositionX>,
        LinearMovementBundle<CutsceneTimeDomain, TargetingPositionY>,
    ) {
        let normalised_direction = (self.position - coordinates).normalize_or_zero();
        let velocity = normalised_direction * self.speed * GAME_BASE_SPEED;

        (
            LinearMovementBundle::<CutsceneTimeDomain, TargetingPositionX>::new(
                coordinates.x,
                self.position.x,
                velocity.x,
            ),
            LinearMovementBundle::<CutsceneTimeDomain, TargetingPositionY>::new(
                coordinates.y,
                self.position.y,
                velocity.y,
            ),
        )
    }
}

#[serde_as]
#[cfg_attr(feature = "derive-ts", derive(TS))]
#[cfg_attr(feature = "derive-ts", ts(export))]
#[derive(new, Clone, Debug, Deserialize, Reflect, Serialize)]
pub struct CutsceneAnimationSpawn {
    pub image_path: String,
    pub frame_count: usize,
    #[serde_as(as = "DurationSecondsWithFrac")]
    #[cfg_attr(feature = "derive-ts", ts(type = "number"))]
    pub duration: Duration,
    #[cfg_attr(feature = "derive-ts", ts(type = "any"))]
    pub layer: Layer,
    #[new(default)]
    #[serde(default)]
    #[cfg_attr(feature = "derive-ts", ts(type = "[number, number]"))]
    pub coordinates: Vec2,
    #[new(default)]
    #[serde(default)]
    pub tag_o: Option<String>,
    #[new(default)]
    #[serde(default)]
    pub target_movement_o: Option<TargetMovement>,
}

#[cfg_attr(feature = "derive-ts", derive(TS))]
#[cfg_attr(feature = "derive-ts", ts(export))]
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
}

#[derive(new, Clone, Component, Debug, Deserialize, Reflect, Serialize)]
pub struct CutsceneImagesSpawn {
    #[new(default)]
    pub spawns: Vec<CutsceneImageSpawn>,
}

impl CutsceneImagesSpawn {
    pub fn push_spawn(mut self, spawn: CutsceneImageSpawn) -> Self {
        self.spawns.push(spawn);
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
}

impl CutsceneAct {
    pub fn move_letterbox(mut self, x: LetterboxMove) -> Self {
        self.letterbox_move_o = Some(x);
        self
    }

    pub fn spawn_animations(mut self, spawns: CutsceneAnimationsSpawn) -> Self {
        self.spawn_animations_o = Some(spawns);
        self
    }

    pub fn spawn_images(mut self, spawns: CutsceneImagesSpawn) -> Self {
        self.spawn_images_o = Some(spawns);
        self
    }

    pub fn spawn_music(mut self, spawn: CutsceneMusicSpawn) -> Self {
        self.music_spawn_o = Some(spawn);
        self
    }

    pub fn despawn_music(mut self) -> Self {
        self.music_despawn_o = Some(CutsceneMusicDespawn {});
        self
    }

    pub fn with_elapse(mut self, secs: f32) -> Self {
        self.elapse = Duration::from_secs_f32(secs);
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
}

impl CutsceneData {
    pub fn set_steps(mut self, steps: Vec<CutsceneAct>) -> Self {
        self.steps = steps;
        self
    }
}
