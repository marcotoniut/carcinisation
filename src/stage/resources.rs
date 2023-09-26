use std::time::Duration;

use bevy::prelude::*;

use crate::plugins::linear::movement::components::DeltaTime;

use super::data::StageData;

#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct StageTime {
    pub delta: Duration,
    pub elapsed: Duration,
}

impl DeltaTime for StageTime {
    fn delta_seconds(&self) -> f32 {
        self.delta.as_secs_f32()
    }
}

#[derive(Clone, Debug, Default, Resource)]
pub struct StageProgress {
    pub elapsed: f32,
    pub step: usize,
    pub step_elapsed: f32,
    pub spawn_step: usize,
    pub spawn_step_elapsed: f32,
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

#[derive(Resource)]
pub struct StageDataHandle(pub Handle<StageData>);
