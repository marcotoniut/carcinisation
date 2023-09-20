use bevy::{
    prelude::Resource,
    time::{Timer, TimerMode},
};
use serde::Deserialize;

#[derive(Debug, Deserialize, Resource, Clone, Default)]
pub struct StageProgress {
    pub elapsed: f32,
    pub step: usize,
    pub step_elapsed: f32,
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
