//! Shared components that describe stage state, scripted steps, and runtime markers.

pub mod damage;
pub mod interactive;
pub mod placement;

use super::{
    data::{ContainerSpawn, StageSpawn},
    floors::SurfaceSpec,
    projection::ProjectionProfile,
};
use crate::cutscene::data::CutsceneAnimationsSpawn;
use bevy::prelude::*;
use derive_new::new;
use serde::{Deserialize, Serialize};
use serde_with::{DurationSecondsWithFrac, serde_as};
use std::time::Duration;

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
    #[must_use]
    pub fn from_secs_f32(secs: f32) -> Self {
        Self {
            duration: Duration::from_secs_f32(secs),
            clear_graphics: false,
        }
    }

    /// Flags that any graphics created during the elapse should be cleaned up.
    #[must_use]
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
/// Tween segment describing coordinates, base speed, and spawns for the step.
pub struct TweenStageStep {
    #[new(default)]
    pub coordinates: Vec2,
    #[new(value = "1.")]
    #[serde(default = "default_base_speed")]
    pub base_speed: f32,
    #[new(default)]
    #[serde(default)]
    pub spawns: Vec<StageSpawn>,
    /// Surface declarations for this step.
    ///
    /// `None` = inherit from the most recent step that declared surfaces.
    /// `Some(vec![])` = explicitly no surfaces (topology removal).
    /// `Some(vec![...])` = declare surfaces for this step.
    #[new(default)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surfaces: Option<Vec<SurfaceSpec>>,
    /// Step-specific projection override.  When present, replaces the stage
    /// default for this step.  During tween steps, runtime interpolates from
    /// the previous effective profile to this one.
    #[new(default)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projection: Option<ProjectionProfile>,
    /// Parallax attenuation multiplier for this step.
    ///
    /// `0.0` = no parallax, `1.0` = full parallax.  `None` = inherit from
    /// the most recent step that set a value (sticky carry-forward), falling
    /// back to [`StageData::parallax_attenuation`], then to `1.0`.
    ///
    /// During tween steps, runtime interpolates linearly from the previous
    /// effective value to this one.
    #[new(default)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parallax_attenuation: Option<f32>,
    // TODO
    // pub is_checkpoint: bool,
    // pub music_fade: bool,
    // pub music_track: Option<String>,
}

impl TweenStageStep {
    /// Appends extra spawns to the step definition.
    #[must_use]
    pub fn add_spawns(mut self, new_spawns: Vec<StageSpawn>) -> Self {
        self.spawns.extend(new_spawns);
        self
    }

    /// Base builder with initial coordinates.
    #[must_use]
    pub fn base(x: f32, y: f32) -> Self {
        Self::new().with_coordinates(Vec2::new(x, y))
    }

    /// Overrides the base tween speed used for the segment.
    #[must_use]
    pub fn with_base_speed(mut self, value: f32) -> Self {
        self.base_speed = value;
        self
    }

    /// Sets the coordinates the stage camera/entity should aim for.
    #[must_use]
    pub fn with_coordinates(mut self, value: Vec2) -> Self {
        self.coordinates = value;
        self
    }

    /// Sets the surface declarations for this tween step.
    #[must_use]
    pub fn with_surfaces(mut self, value: Vec<SurfaceSpec>) -> Self {
        self.surfaces = Some(value);
        self
    }

    /// Overrides the projection profile for this tween step.
    #[must_use]
    pub fn with_projection(mut self, value: ProjectionProfile) -> Self {
        self.projection = Some(value);
        self
    }

    /// Overrides the parallax attenuation for this tween step.
    #[must_use]
    pub fn with_parallax_attenuation(mut self, value: f32) -> Self {
        self.parallax_attenuation = Some(value);
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
    /// Surface declarations for this step. See [`TweenStageStep::surfaces`].
    #[new(default)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surfaces: Option<Vec<SurfaceSpec>>,
    /// Step-specific projection override.  See [`TweenStageStep::projection`].
    #[new(default)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projection: Option<ProjectionProfile>,
    /// Parallax attenuation for this step.  See [`TweenStageStep::parallax_attenuation`].
    #[new(default)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parallax_attenuation: Option<f32>,
    // TODO
    // pub is_checkpoint: bool,
    // pub music_fade: bool,
    // pub music_track: Option<String>,
}

impl StopStageStep {
    /// Appends extra spawns that occur during the stop.
    #[must_use]
    pub fn add_spawns(mut self, new_spawns: Vec<StageSpawn>) -> Self {
        self.spawns.extend(new_spawns);
        self
    }

    /// Configures whether the stop step clears all enemies.
    #[must_use]
    pub fn with_kill_all(mut self, value: bool) -> Self {
        self.kill_all = value;
        self
    }

    /// Configures whether the stop step clears the boss.
    #[must_use]
    pub fn with_kill_boss(mut self, value: bool) -> Self {
        self.kill_boss = value;
        self
    }

    /// Sets a maximum duration before the stop advances automatically.
    #[must_use]
    pub fn with_max_duration(mut self, value: f32) -> Self {
        self.max_duration = Some(Duration::from_secs_f32(value));
        self
    }

    /// Sets the surface declarations for this stop step.
    #[must_use]
    pub fn with_surfaces(mut self, value: Vec<SurfaceSpec>) -> Self {
        self.surfaces = Some(value);
        self
    }

    /// Overrides the projection profile for this stop step.
    #[must_use]
    pub fn with_projection(mut self, value: ProjectionProfile) -> Self {
        self.projection = Some(value);
        self
    }

    /// Overrides the parallax attenuation for this stop step.
    #[must_use]
    pub fn with_parallax_attenuation(mut self, value: f32) -> Self {
        self.parallax_attenuation = Some(value);
        self
    }
}
