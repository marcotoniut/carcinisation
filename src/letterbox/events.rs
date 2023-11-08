use bevy::prelude::*;

use super::components::LETTERBOX_HEIGHT;

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
        Self::new(LETTERBOX_HEIGHT as f32, 30.0)
    }

    pub fn close() -> Self {
        Self::new(LETTERBOX_HEIGHT as f32, 0.0)
    }
}
