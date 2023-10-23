use bevy::prelude::{Component, Resource, Vec2};
use std::time::Duration;

use crate::{
    plugins::movement::linear::components::{
        LinearMovementBundle, TargetingPositionX, TargetingPositionY,
    },
    stage::data::GAME_BASE_SPEED,
    Layer,
};

use super::resources::CutsceneTime;

#[derive(Clone, Debug, PartialEq, PartialOrd, Eq, Ord)]
pub enum CutsceneLayer {
    Background(u8),
    Middle(u8),
    Letterbox,
    Foreground(u8),
    Textbox,
    Text,
    Overtext(u8),
}

#[derive(Clone, Copy, Debug)]
pub struct TargetMovement {
    pub position: Vec2,
    pub speed: f32,
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
        let normalised_direction = (self.position - coordinates).normalize();
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

#[derive(Clone, Debug)]
pub struct CutsceneAnimationSpawn {
    pub duration: Duration,
    pub frame_count: usize,
    pub image_path: String,
    pub coordinates: Vec2,
    pub layer: Layer,
    pub tag_o: Option<String>,
    pub target_movement_o: Option<TargetMovement>,
}

impl CutsceneAnimationSpawn {
    pub fn new(image_path: String, frame_count: usize, secs: f32) -> Self {
        Self {
            duration: Duration::from_secs_f32(secs),
            frame_count,
            image_path,
            coordinates: Vec2::ZERO,
            layer: Layer::CutsceneLayer(CutsceneLayer::Background(0)),
            tag_o: None,
            target_movement_o: None,
        }
    }

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

#[derive(Clone, Debug, Component)]
pub struct CutsceneAnimationsSpawn {
    pub spawns: Vec<CutsceneAnimationSpawn>,
}

impl CutsceneAnimationsSpawn {
    pub fn new() -> Self {
        Self { spawns: vec![] }
    }

    pub fn push_spawn(mut self, spawn: CutsceneAnimationSpawn) -> Self {
        self.spawns.push(spawn);
        self
    }
}

#[derive(Clone, Debug, Component)]
pub struct CutsceneAwaitInput;

impl CutsceneAwaitInput {
    pub fn new() -> Self {
        Self {}
    }
}

#[derive(Clone, Debug, Component)]
pub struct CutsceneElapse {
    pub duration: Duration,
    pub clear_graphics: bool,
}

impl CutsceneElapse {
    pub fn new(duration: Duration) -> Self {
        Self {
            duration,
            clear_graphics: false,
        }
    }

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

#[derive(Clone, Debug)]
pub struct CutsceneImageSpawn {
    pub image_path: String,
    pub coordinates: Vec2,
    pub layer: Layer,
    pub tag_o: Option<String>,
}

impl CutsceneImageSpawn {
    pub fn new(image_path: String) -> Self {
        Self {
            image_path,
            coordinates: Vec2::ZERO,
            layer: Layer::CutsceneLayer(CutsceneLayer::Background(0)),
            tag_o: None,
        }
    }

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

#[derive(Clone, Debug, Component)]
pub struct CutsceneImagesSpawn {
    pub spawns: Vec<CutsceneImageSpawn>,
}

impl CutsceneImagesSpawn {
    pub fn new() -> Self {
        Self { spawns: vec![] }
    }

    pub fn push_spawn(mut self, spawn: CutsceneImageSpawn) -> Self {
        self.spawns.push(spawn);
        self
    }
}

#[derive(Clone, Debug)]
pub struct CutsceneAct {
    pub await_input: bool,
    pub despawn_entities: Vec<String>,
    pub elapse: Duration,
    pub music_despawn_o: Option<CutsceneMusicDespawn>,
    pub music_spawn_o: Option<CutsceneMusicSpawn>,
    pub spawn_animations_o: Option<CutsceneAnimationsSpawn>,
    pub spawn_images_o: Option<CutsceneImagesSpawn>,
}

impl CutsceneAct {
    pub fn new() -> Self {
        Self {
            await_input: false,
            despawn_entities: vec![],
            elapse: Duration::ZERO,
            music_despawn_o: None,
            music_spawn_o: None,
            spawn_animations_o: None,
            spawn_images_o: None,
        }
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

#[derive(Clone, Debug, Component)]
pub struct CutsceneMusicSpawn {
    pub music_path: String,
    // TODO fade_in
}

impl CutsceneMusicSpawn {
    pub fn new(music_path: String) -> Self {
        Self { music_path }
    }
}

#[derive(Clone, Debug, Component)]
pub struct CutsceneMusicDespawn {
    // TODO fade_out
}

impl CutsceneMusicDespawn {
    pub fn new() -> Self {
        Self {}
    }
}

#[derive(Clone, Debug, Component)]
pub struct CutsceneSpriteSpawn {
    pub image_path: String,
    pub coordinates: Vec2,
    pub tag_o: Option<String>,
}

impl CutsceneSpriteSpawn {
    pub fn new(image_path: String, coordinates: Vec2) -> Self {
        Self {
            image_path,
            coordinates,
            tag_o: None,
        }
    }
}

#[derive(Clone, Debug, Resource)]
pub struct CutsceneData {
    pub name: String,
    pub steps: Vec<CutsceneAct>,
}
