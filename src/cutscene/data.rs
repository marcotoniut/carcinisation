use super::resources::CutsceneTime;
use crate::{
    layer::Layer,
    letterbox::events::LetterboxMove,
    plugins::movement::linear::components::{
        LinearMovementBundle, TargetingPositionX, TargetingPositionY,
    },
    stage::data::GAME_BASE_SPEED,
};
use bevy::prelude::*;
use derive_new::new;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationSecondsWithFrac};
use std::time::Duration;

#[derive(Clone, Debug, Deserialize, PartialEq, PartialOrd, Eq, Ord, Reflect, Serialize)]
pub enum CutsceneLayer {
    Background(u8),
    Middle(u8),
    Letterbox,
    Foreground(u8),
    Textbox,
    Text,
    Overtext(u8),
}

impl Default for CutsceneLayer {
    fn default() -> Self {
        CutsceneLayer::Background(0)
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Reflect, Serialize)]
#[serde_as]
pub struct TargetMovement {
    pub position: Vec2,
    #[serde(default)]
    pub speed: f32,
    #[serde(default)]
    pub acceleration: f32,
}

impl TargetMovement {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            position: Vec2::new(x, y),
            speed: 0.,
            acceleration: 0.,
        }
    }

    pub fn with_speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }

    pub fn with_acceleration(mut self, acceleration: f32) -> Self {
        self.acceleration = acceleration;
        self
    }

    pub fn make_bundles(
        self,
        coordinates: Vec2,
    ) -> (
        LinearMovementBundle<CutsceneTime, TargetingPositionX>,
        LinearMovementBundle<CutsceneTime, TargetingPositionY>,
    ) {
        let normalised_direction = (self.position - coordinates).normalize_or_zero();
        let velocity = normalised_direction * self.speed * GAME_BASE_SPEED;

        (
            LinearMovementBundle::<CutsceneTime, TargetingPositionX>::new(
                coordinates.x,
                self.position.x,
                velocity.x,
            ),
            LinearMovementBundle::<CutsceneTime, TargetingPositionY>::new(
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
}

impl CutsceneAnimationSpawn {
    pub fn with_coordinates(mut self, x: f32, y: f32) -> Self {
        self.coordinates = Vec2::new(x, y);
        self
    }

    pub fn with_layer(mut self, layer: Layer) -> Self {
        self.layer = layer;
        self
    }

    pub fn with_tag(mut self, tag: String) -> Self {
        self.tag_o = Some(tag);
        self
    }

    pub fn with_target_movement(mut self, target_movement: TargetMovement) -> Self {
        self.target_movement_o = Some(target_movement);
        self
    }
}

#[derive(new, Clone, Component, Debug, Deserialize, Reflect, Serialize)]
pub struct CutsceneAnimationsSpawn {
    #[new(default)]
    pub spawns: Vec<CutsceneAnimationSpawn>,
}

impl CutsceneAnimationsSpawn {
    pub fn push_spawn(mut self, spawn: CutsceneAnimationSpawn) -> Self {
        self.spawns.push(spawn);
        self
    }
}

#[derive(new, Clone, Debug, Component)]
pub struct CutsceneAwaitInput;

#[derive(new, Clone, Debug, Component)]
pub struct CutsceneElapse {
    pub duration: Duration,
    #[new(value = "false")]
    pub clear_graphics: bool,
}

impl CutsceneElapse {
    pub fn from_secs_f32(secs: f32) -> Self {
        Self {
            duration: Duration::from_secs_f32(secs),
            clear_graphics: false,
        }
    }

    pub fn clear_graphics(mut self) -> Self {
        self.clear_graphics = true;
        self
    }
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

impl CutsceneImageSpawn {
    pub fn with_coordinates(mut self, x: f32, y: f32) -> Self {
        self.coordinates = Vec2::new(x, y);
        self
    }

    pub fn with_layer(mut self, layer: Layer) -> Self {
        self.layer = layer;
        self
    }

    pub fn with_tag(mut self, tag: String) -> Self {
        self.tag_o = Some(tag);
        self
    }
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
    #[serde_as(as = "DurationSecondsWithFrac")]
    #[serde(default)]
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
pub struct CutsceneTransition;

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
