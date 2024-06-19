use bevy::prelude::*;
use derive_new::new;
use serde::{Deserialize, Serialize};

use super::components::{LETTERBOX_HEIGHT, LETTERBOX_INSTANT_SPEED, LETTERBOX_NORMAL_SPEED};

#[derive(new, Event, Clone, Debug, Deserialize, Serialize)]
pub struct LetterboxMoveEvent {
    pub speed: f32,
    pub target: f32,
}

impl LetterboxMoveEvent {
    pub fn open() -> Self {
        Self::new(LETTERBOX_NORMAL_SPEED, LETTERBOX_HEIGHT as f32)
    }

    pub fn close() -> Self {
        Self::new(LETTERBOX_NORMAL_SPEED, 0.0)
    }

    pub fn show() -> Self {
        Self::new(LETTERBOX_INSTANT_SPEED, LETTERBOX_HEIGHT as f32)
    }

    pub fn hide() -> Self {
        Self::new(LETTERBOX_INSTANT_SPEED, 0.0)
    }

    pub fn move_to(target: f32) -> Self {
        Self::new(LETTERBOX_NORMAL_SPEED, target)
    }

    pub fn move_to_at(target: f32, speed: f32) -> Self {
        Self::new(speed, target)
    }
}
