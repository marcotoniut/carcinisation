//! Messages for animating the letterbox UI.

use bevy::prelude::*;
use derive_new::new;
use serde::{Deserialize, Serialize};

use super::components::{LETTERBOX_HEIGHT, LETTERBOX_INSTANT_SPEED, LETTERBOX_NORMAL_SPEED};
use carcinisation_cutscene::data::LetterboxMove;

#[derive(new, Clone, Debug, Deserialize, Event, Message, Serialize)]
/// Motion command that drives letterbox movement.
pub struct LetterboxMoveEvent {
    pub speed: f32,
    pub target: f32,
}

impl LetterboxMoveEvent {
    /// Slides bars to the open position using default speed.
    #[must_use]
    pub fn open() -> Self {
        Self::new(LETTERBOX_NORMAL_SPEED, LETTERBOX_HEIGHT as f32)
    }

    /// Slides bars to the closed position using default speed.
    #[must_use]
    pub fn close() -> Self {
        Self::new(LETTERBOX_NORMAL_SPEED, 0.0)
    }

    /// Instantly shows the bars.
    #[must_use]
    pub fn show() -> Self {
        Self::new(LETTERBOX_INSTANT_SPEED, LETTERBOX_HEIGHT as f32)
    }

    /// Instantly hides the bars.
    #[must_use]
    pub fn hide() -> Self {
        Self::new(LETTERBOX_INSTANT_SPEED, 0.0)
    }

    /// Moves bars to a target using default speed.
    #[must_use]
    pub fn move_to(target: f32) -> Self {
        Self::new(LETTERBOX_NORMAL_SPEED, target)
    }

    /// Moves bars to a target using the provided speed.
    #[must_use]
    pub fn move_to_at(target: f32, speed: f32) -> Self {
        Self::new(speed, target)
    }
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
