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
}

#[derive(Resource)]
pub struct StageTimer {
    pub timer: Timer,
}

impl Default for StageTimer {
    fn default() -> Self {
        StageTimer {
            timer: Timer::from_seconds(0.0, TimerMode::Repeating),
        }
    }
}
