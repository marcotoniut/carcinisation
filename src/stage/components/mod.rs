pub mod damage;
pub mod interactive;
pub mod placement;

use std::time::Duration;

use bevy::prelude::*;

use crate::cinemachine::data::CinemachineData;

use super::data::{ContainerSpawn, StageSpawn, StageStep};

// TODO should go in UI
#[derive(Clone, Component, Debug)]
pub struct StageClearedText {}

#[derive(Clone, Component, Debug)]
pub struct SpawnDrop {
    pub contains: ContainerSpawn,
    pub entity: Entity,
}

#[derive(Component)]
pub struct Stage;

#[derive(Component)]
pub struct CurrentStageStep {
    pub started: Duration,
}

#[derive(Component, Clone, Debug)]
pub struct CinematicStageStep {
    pub cinematic: CinemachineData,
}

#[derive(Component, Clone, Debug)]
pub struct MovementStageStep {
    pub coordinates: Vec2,
    pub base_speed: f32,
    pub spawns: Vec<StageSpawn>,
    // TODO
    // pub is_checkpoint: bool,
    // pub music_fade: bool,
    // pub music_track: Option<String>,
}

#[derive(Component, Clone, Debug)]
pub struct StopStageStep {
    pub max_duration: Option<Duration>,
    pub kill_all: bool,
    pub kill_boss: bool,
    pub spawns: Vec<StageSpawn>,
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
}
