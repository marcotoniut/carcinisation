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

#[derive(Clone, Debug, Deserialize, Reflect, Serialize)]
pub enum LetterboxMove {
    To(f32),
    ToAt(f32, f32),
    Hide,
    Show,
    Close,
    Open,
}

impl From<LetterboxMove> for LetterboxMoveEvent {
    fn from(x: LetterboxMove) -> Self {
        match x {
            LetterboxMove::To(target) => LetterboxMoveEvent::move_to(target),
            LetterboxMove::ToAt(target, speed) => LetterboxMoveEvent::move_to_at(target, speed),
            LetterboxMove::Hide => LetterboxMoveEvent::hide(),
            LetterboxMove::Show => LetterboxMoveEvent::show(),
            LetterboxMove::Close => LetterboxMoveEvent::close(),
            LetterboxMove::Open => LetterboxMoveEvent::open(),
        }
    }
}

impl From<LetterboxMoveEvent> for LetterboxMove {
    fn from(e: LetterboxMoveEvent) -> Self {
        let LetterboxMoveEvent { target, speed } = e;
        LetterboxMove::ToAt(target, speed)
    }
}
