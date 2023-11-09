use bevy::prelude::*;

use super::components::{LETTERBOX_HEIGHT, LETTERBOX_NORMAL_SPEED};

#[derive(Event, Clone, Debug)]
pub struct LetterboxMoveEvent {
    pub speed: f32,
    pub target: f32,
}

impl LetterboxMoveEvent {
    pub fn new(speed: f32, target: f32) -> Self {
        Self { speed, target }
    }

    pub fn open() -> Self {
        Self::new(LETTERBOX_NORMAL_SPEED, LETTERBOX_HEIGHT as f32)
    }

    pub fn close() -> Self {
        Self::new(LETTERBOX_NORMAL_SPEED, 0.0)
    }

    pub fn show() -> Self {
        Self::new(LETTERBOX_NORMAL_SPEED, LETTERBOX_HEIGHT as f32)
    }

    pub fn hide() -> Self {
        Self::new(LETTERBOX_NORMAL_SPEED, 0.0)
    }

    pub fn move_to(target: f32) -> Self {
        Self::new(LETTERBOX_NORMAL_SPEED, target)
    }

    pub fn move_to_at(target: f32, speed: f32) -> Self {
        Self::new(speed, target)
    }
}
