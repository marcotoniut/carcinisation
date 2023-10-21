use super::components::LETTERBOX_UPDATE_TIME;
use crate::core::time::*;
use bevy::{
    prelude::Resource,
    time::{Timer, TimerMode},
};
use std::time::Duration;

#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct CutsceneTime {
    pub delta: Duration,
    pub elapsed: Duration,
}

impl DeltaTime for CutsceneTime {
    fn delta_seconds(&self) -> f32 {
        self.delta.as_secs_f32()
    }

    fn delta(&self) -> Duration {
        self.delta
    }
}

impl ElapsedTime for CutsceneTime {
    fn elapsed(&self) -> Duration {
        self.elapsed
    }
}

impl Ticker for CutsceneTime {
    fn tick(&mut self, delta: Duration) {
        self.delta = delta;
        self.elapsed += delta;
    }
}

#[derive(Resource, Default, Clone, Copy)]
pub struct CutsceneProgress {
    pub index: usize,
}

#[derive(Resource)]
pub struct LetterboxUpdateTimer {
    pub timer: Timer,
}

#[derive(Resource, Default)]
pub struct LetterboxCounter {
    pub value: u32,
}

impl Default for LetterboxUpdateTimer {
    fn default() -> Self {
        LetterboxUpdateTimer {
            timer: Timer::from_seconds(LETTERBOX_UPDATE_TIME, TimerMode::Repeating),
        }
    }
}
