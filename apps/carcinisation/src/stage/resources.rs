//! Stage-scoped resources for tracking time, progress, and spawn timers.

use std::time::Duration;

use bevy::prelude::*;
use derive_new::new;

use super::data::StageSpawn;

#[derive(Resource, Default, Clone, Copy, Debug)]
/// Marker used to scope Bevy's `Time` to the active stage.
pub struct StageTimeDomain;

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

impl StageActionTimer {
    pub fn start(&mut self, duration: Duration) {
        self.timer.set_duration(duration);
        self.timer.reset();
        self.timer.unpause();
    }

    pub fn stop(&mut self) {
        self.timer.pause();
    }
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
