use bevy::{
    prelude::Resource,
    time::{Timer, TimerMode},
};

use super::components::LETTERBOX_UPDATE_TIME;

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
