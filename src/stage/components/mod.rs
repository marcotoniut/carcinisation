pub mod damage;
pub mod interactive;
pub mod placement;

use self::placement::Depth;
use super::data::{ContainerSpawn, StageSpawn};
use crate::cutscene::data::CutsceneAnimationsSpawn;
use bevy::{prelude::*, utils::HashMap};
use derive_new::new;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationSecondsWithFrac};
use std::time::Duration;

#[derive(Component, Debug, Default)]
pub struct StageEntity;

// TODO should go in UI
#[derive(Clone, Component, Debug)]
pub struct StageClearedText;

#[derive(Clone, Component, Debug)]
pub struct SpawnDrop {
    pub contains: ContainerSpawn,
    pub entity: Entity,
}

#[derive(Component)]
pub struct Stage;

#[derive(Component, Reflect)]
pub struct CurrentStageStep {
    pub started: Duration,
}

// TODO use this instead of CurrentStageStep?
#[derive(new, Clone, Debug, Component, Reflect)]
pub struct StageElapse {
    pub duration: Duration,
    #[new(default)]
    pub clear_graphics: bool,
}

impl StageElapse {
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

#[derive(Component, Reflect)]
pub struct StageElapsedStarted(pub Duration);

#[derive(Component, Clone, Debug, Deserialize, Reflect, Serialize)]
pub enum CinematicStageStep {
    CutsceneAnimationSpawn(CutsceneAnimationsSpawn),
}

#[derive(new, Component, Clone, Debug, Deserialize, Reflect, Serialize)]
pub struct MovementStageStep {
    #[new(default)]
    pub coordinates: Vec2,
    #[new(value = "1.")]
    pub base_speed: f32,
    #[new(default)]
    pub spawns: Vec<StageSpawn>,
    #[new(default)]
    pub floor_depths: Option<HashMap<Depth, f32>>,
    // TODO
    // pub is_checkpoint: bool,
    // pub music_fade: bool,
    // pub music_track: Option<String>,
}

impl MovementStageStep {
    pub fn add_spawns(mut self, new_spawns: Vec<StageSpawn>) -> Self {
        self.spawns.extend(new_spawns);
        self
    }

    pub fn base(x: f32, y: f32) -> Self {
        Self::new().with_coordinates(Vec2::new(x, y))
    }

    pub fn with_base_speed(mut self, value: f32) -> Self {
        self.base_speed = value;
        self
    }

    pub fn with_coordinates(mut self, value: Vec2) -> Self {
        self.coordinates = value;
        self
    }

    pub fn with_floor_depths(mut self, value: HashMap<Depth, f32>) -> Self {
        self.floor_depths = Some(value);
        self
    }
}

#[serde_as]
#[derive(new, Component, Clone, Debug, Deserialize, Reflect, Serialize)]
pub struct StopStageStep {
    #[new(default)]
    #[serde_as(as = "Option<DurationSecondsWithFrac>")]
    #[serde(default)]
    pub max_duration: Option<Duration>,
    #[new(value = "true")]
    pub kill_all: bool,
    #[new(default)]
    #[serde(default)]
    pub kill_boss: bool,
    #[new(default)]
    #[serde(default)]
    pub spawns: Vec<StageSpawn>,
    #[new(default)]
    #[serde(default)]
    pub floor_depths: Option<HashMap<Depth, f32>>,
    // TODO
    // pub is_checkpoint: bool,
    // pub music_fade: bool,
    // pub music_track: Option<String>,
}

impl StopStageStep {
    pub fn add_spawns(mut self, new_spawns: Vec<StageSpawn>) -> Self {
        self.spawns.extend(new_spawns);
        self
    }

    pub fn with_kill_all(mut self, value: bool) -> Self {
        self.kill_all = value;
        self
    }

    pub fn with_kill_boss(mut self, value: bool) -> Self {
        self.kill_boss = value;
        self
    }

    pub fn with_max_duration(mut self, value: f32) -> Self {
        self.max_duration = Some(Duration::from_secs_f32(value));
        self
    }

    pub fn with_floor_depths(mut self, value: HashMap<Depth, f32>) -> Self {
        self.floor_depths = Some(value);
        self
    }
}
