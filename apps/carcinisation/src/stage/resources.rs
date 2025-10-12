//! Stage-scoped resources for tracking time, progress, and spawn timers.

use std::time::Duration;

use bevy::prelude::*;
use derive_new::new;

use crate::core::time::*;

use super::data::StageSpawn;

#[derive(Resource, Default, Debug, Clone, Copy, Reflect)]
/// Accumulates stage delta/elapsed time for time-based systems.
pub struct StageTime {
    pub delta: Duration,
    pub elapsed: Duration,
}

impl DeltaTime for StageTime {
    fn delta(&self) -> Duration {
        self.delta
    }
}

impl ElapsedTime for StageTime {
    fn elapsed(&self) -> Duration {
        self.elapsed
    }
}

impl Ticker for StageTime {
    fn tick(&mut self, delta: Duration) {
        self.delta = delta;
        self.elapsed += delta;
    }
}

#[derive(Clone, Debug, Default, Resource)]
/// Stores the active stage step index.
pub struct StageProgress {
    pub index: usize,
}

#[derive(Resource)]
/// Wrapper timer used to pace scripted stage actions.
pub struct StageActionTimer {
    pub timer: Timer,
}

impl Default for StageActionTimer {
    fn default() -> Self {
        let mut timer = Timer::from_seconds(0., TimerMode::Once);
        timer.pause();
        StageActionTimer { timer }
    }
}

#[derive(new, Component, Default)]
/// Component that sequences stage spawns and tracks elapsed times.
pub struct StageStepSpawner {
    #[new(default)]
    pub elapsed: Duration,
    #[new(default)]
    pub elapsed_since_spawn: Duration,
    pub spawns: Vec<StageSpawn>,
}
