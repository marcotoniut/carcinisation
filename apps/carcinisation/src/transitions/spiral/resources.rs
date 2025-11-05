use std::time::Duration;

use bevy::{
    prelude::Resource,
    time::{Timer, TimerMode},
};

use crate::core::time::{DeltaTime, ElapsedTime, Ticker};

use super::components::TRANSITION_UPDATE_TIME;

#[derive(Resource)]
pub struct TransitionVenetianTime {
    pub delta: Duration,
    pub elapsed: Duration,
}

impl Default for TransitionVenetianTime {
    fn default() -> Self {
        Self {
            delta: Duration::ZERO,
            elapsed: Duration::ZERO,
        }
    }
}

impl DeltaTime for TransitionVenetianTime {
    fn delta(&self) -> Duration {
        self.delta
    }
}

impl ElapsedTime for TransitionVenetianTime {
    fn elapsed(&self) -> Duration {
        self.elapsed
    }
}

impl Ticker for TransitionVenetianTime {
    fn tick(&mut self, delta: Duration) {
        self.delta = delta;
        self.elapsed += delta;
    }
}

#[derive(Resource)]
pub struct TransitionUpdateTimer {
    pub timer: Timer,
}

#[derive(Resource, Default)]
pub struct TransitionCounter {
    pub value: u32,
    pub finished: bool,
}

impl Default for TransitionUpdateTimer {
    fn default() -> Self {
        TransitionUpdateTimer {
            timer: Timer::from_seconds(TRANSITION_UPDATE_TIME, TimerMode::Repeating),
        }
    }
}
