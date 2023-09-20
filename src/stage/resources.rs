use bevy::{
    prelude::Resource,
    time::{Timer, TimerMode},
};
use serde::Deserialize;

// TODO goes under game mod,
//  REVIEW should split?
#[derive(Debug, Deserialize, Resource, Clone, Default)]
pub struct GameProgress {
    pub stage: usize,
    pub stage_step: usize,
    pub last_step_started: f32,
}

#[derive(Resource)]
pub struct StageTimer {
    pub timer: Timer,
}

impl Default for StageTimer {
    fn default() -> Self {
        let timer = Timer::from_seconds(0., TimerMode::Repeating);
        StageTimer { timer }
    }
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
