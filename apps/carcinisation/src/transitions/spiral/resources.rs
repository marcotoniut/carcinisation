use bevy::{
    prelude::Resource,
    time::{Timer, TimerMode},
};

use super::components::TRANSITION_UPDATE_TIME;

#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct TransitionVenetianTimeDomain;

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
