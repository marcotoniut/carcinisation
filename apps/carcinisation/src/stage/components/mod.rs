//! Shared components that describe stage state, scripted steps, and runtime markers.

pub mod damage;
pub mod interactive;
pub mod placement;

use self::placement::Depth;
use super::data::{ContainerSpawn, StageSpawn};
use crate::cutscene::data::CutsceneAnimationsSpawn;
use bevy::prelude::*;
use derive_new::new;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationSecondsWithFrac};
use std::{collections::HashMap, time::Duration};

#[derive(Component, Debug, Default)]
/// Marker for entities that belong to the current stage run.
pub struct StageEntity;

// TODO should go in UI
#[derive(Clone, Component, Debug)]
/// UI marker for the "Stage Cleared" text element.
pub struct StageClearedText;

#[derive(Clone, Component, Debug)]
/// Tracks a pending drop to spawn after an entity dies.
pub struct SpawnDrop {
    pub contains: ContainerSpawn,
    pub entity: Entity,
}

#[derive(Component)]
/// Marks the root entity for the active stage scene.
pub struct Stage;

#[derive(Component, Reflect)]
/// Timestamp of when the current scripted step began.
pub struct CurrentStageStep {
    pub started: Duration,
}

// TODO use this instead of CurrentStageStep?
#[derive(new, Clone, Debug, Component, Reflect)]
/// Describes a timed stage elapse segment and whether it clears graphics on completion.
pub struct StageElapse {
    pub duration: Duration,
    #[new(default)]
    pub clear_graphics: bool,
}

impl StageElapse {
    /// Convenience constructor from seconds.
    pub fn from_secs_f32(secs: f32) -> Self {
        Self {
            duration: Duration::from_secs_f32(secs),
            clear_graphics: false,
        }
    }

    /// Flags that any graphics created during the elapse should be cleaned up.
    pub fn clear_graphics(mut self) -> Self {
        self.clear_graphics = true;
        self
    }
}

#[derive(Component, Reflect)]
/// Helper component recording the start time of a stage elapse.
pub struct StageElapsedStarted(pub Duration);

#[derive(Component, Clone, Debug, Deserialize, Reflect, Serialize)]
/// Scripted cinematic step triggered during stage progression.
pub enum CinematicStageStep {
    CutsceneAnimationSpawn(CutsceneAnimationsSpawn),
}

fn default_base_speed() -> f32 {
    1.0
}

#[derive(new, Component, Clone, Debug, Deserialize, Reflect, Serialize)]
/// Movement segment describing coordinates, base speed, and spawns for the step.
pub struct MovementStageStep {
    #[new(default)]
    pub coordinates: Vec2,
    #[new(value = "1.")]
    #[serde(default = "default_base_speed")]
    pub base_speed: f32,
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

impl MovementStageStep {
    /// Appends extra spawns to the step definition.
    pub fn add_spawns(mut self, new_spawns: Vec<StageSpawn>) -> Self {
        self.spawns.extend(new_spawns);
        self
    }

    /// Base builder with initial coordinates.
    pub fn base(x: f32, y: f32) -> Self {
        Self::new().with_coordinates(Vec2::new(x, y))
    }

    /// Overrides the base movement speed used for the segment.
    pub fn with_base_speed(mut self, value: f32) -> Self {
        self.base_speed = value;
        self
    }

    /// Sets the coordinates the stage camera/entity should aim for.
    pub fn with_coordinates(mut self, value: Vec2) -> Self {
        self.coordinates = value;
        self
    }

    /// Specifies optional per-depth floor offsets.
    pub fn with_floor_depths(mut self, value: HashMap<Depth, f32>) -> Self {
        self.floor_depths = Some(value);
        self
    }
}

#[serde_as]
#[derive(new, Component, Clone, Debug, Deserialize, Reflect, Serialize)]
/// Stop segment that keeps the stage static until conditions are met.
pub struct StopStageStep {
    #[new(default)]
    #[serde_as(as = "Option<DurationSecondsWithFrac>")]
    #[serde(default)]
    pub max_duration: Option<Duration>,
    #[new(value = "true")]
    #[serde(default)]
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
    /// Appends extra spawns that occur during the stop.
    pub fn add_spawns(mut self, new_spawns: Vec<StageSpawn>) -> Self {
        self.spawns.extend(new_spawns);
        self
    }

    /// Configures whether the stop step clears all enemies.
    pub fn with_kill_all(mut self, value: bool) -> Self {
        self.kill_all = value;
        self
    }

    /// Configures whether the stop step clears the boss.
    pub fn with_kill_boss(mut self, value: bool) -> Self {
        self.kill_boss = value;
        self
    }

    /// Sets a maximum duration before the stop advances automatically.
    pub fn with_max_duration(mut self, value: f32) -> Self {
        self.max_duration = Some(Duration::from_secs_f32(value));
        self
    }

    /// Overrides per-depth floor offsets for the stop step.
    pub fn with_floor_depths(mut self, value: HashMap<Depth, f32>) -> Self {
        self.floor_depths = Some(value);
        self
    }
}
