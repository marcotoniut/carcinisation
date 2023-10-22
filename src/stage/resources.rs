use std::time::Duration;

use bevy::prelude::*;

use crate::core::time::*;

use super::data::StageSpawn;

#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct StageTime {
    pub delta: Duration,
    pub elapsed: Duration,
}

impl DeltaTime for StageTime {
    fn delta_seconds(&self) -> f32 {
        self.delta.as_secs_f32()
    }

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
pub struct StageProgress {
    pub index: usize,
}

#[derive(Resource)]
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

#[derive(Component, Default)]
pub struct StageStepSpawner {
    pub elapsed: Duration,
    pub elapsed_since_spawn: Duration,
    pub spawns: Vec<StageSpawn>,
}

impl StageStepSpawner {
    pub fn new(spawns: Vec<StageSpawn>) -> Self {
        StageStepSpawner {
            spawns,
            ..Default::default()
        }
    }
}
