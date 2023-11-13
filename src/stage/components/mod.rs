pub mod damage;
pub mod interactive;
pub mod placement;

use crate::cutscene::data::CutsceneAnimationsSpawn;

use self::placement::Depth;

use super::data::{ContainerSpawn, StageSpawn};
use bevy::{prelude::*, utils::HashMap};
use std::time::Duration;

#[derive(Component)]
pub struct StageEntity;

pub enum StageEntityType {
    Player,
    Enemy,
    Destructible,
    Attack,
}

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
#[derive(Clone, Debug, Component, Reflect)]
pub struct StageElapse {
    pub duration: Duration,
    pub clear_graphics: bool,
}

impl StageElapse {
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

#[derive(Component, Reflect)]
pub struct StageElapsedStarted(pub Duration);

#[derive(Component, Clone, Debug)]
pub enum CinematicStageStep {
    CutsceneAnimationSpawn(CutsceneAnimationsSpawn),
}

#[derive(Component, Clone, Debug)]
pub struct MovementStageStep {
    pub coordinates: Vec2,
    pub base_speed: f32,
    pub spawns: Vec<StageSpawn>,
    pub floor_depths: Option<HashMap<Depth, f32>>,
    // TODO
    // pub is_checkpoint: bool,
    // pub music_fade: bool,
    // pub music_track: Option<String>,
}

impl MovementStageStep {
    pub fn new() -> Self {
        Self {
            coordinates: Vec2::ZERO,
            base_speed: 0.0,
            spawns: vec![],
            floor_depths: None,
        }
    }

    pub fn add_spawns(mut self, new_spawns: Vec<StageSpawn>) -> Self {
        self.spawns.extend(new_spawns);
        self
    }

    pub fn base(x: f32, y: f32) -> Self {
        Self {
            coordinates: Vec2::new(x, y),
            base_speed: 1.,
            spawns: vec![],
            floor_depths: None,
        }
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

#[derive(Component, Clone, Debug)]
pub struct StopStageStep {
    pub max_duration: Option<Duration>,
    pub kill_all: bool,
    pub kill_boss: bool,
    pub spawns: Vec<StageSpawn>,
    pub floor_depths: Option<HashMap<Depth, f32>>,
    // TODO
    // pub is_checkpoint: bool,
    // pub music_fade: bool,
    // pub music_track: Option<String>,
}

impl StopStageStep {
    pub fn new() -> Self {
        Self {
            kill_all: true,
            kill_boss: false,
            max_duration: None,
            spawns: vec![],
            floor_depths: None,
        }
    }

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
