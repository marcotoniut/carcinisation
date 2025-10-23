//! Events for animating the letterbox UI.

use bevy::prelude::*;
use derive_new::new;
use serde::{Deserialize, Serialize};

use super::components::{LETTERBOX_HEIGHT, LETTERBOX_INSTANT_SPEED, LETTERBOX_NORMAL_SPEED};

#[derive(new, Clone, Debug, Deserialize, Event, Message, Serialize)]
/// Motion command that drives letterbox movement.
pub struct LetterboxMoveTrigger {
    pub speed: f32,
    pub target: f32,
}

impl LetterboxMoveTrigger {
    /// Slides bars to the open position using default speed.
    pub fn open() -> Self {
        Self::new(LETTERBOX_NORMAL_SPEED, LETTERBOX_HEIGHT as f32)
    }

    /// Slides bars to the closed position using default speed.
    pub fn close() -> Self {
        Self::new(LETTERBOX_NORMAL_SPEED, 0.0)
    }

    /// Instantly shows the bars.
    pub fn show() -> Self {
        Self::new(LETTERBOX_INSTANT_SPEED, LETTERBOX_HEIGHT as f32)
    }

    /// Instantly hides the bars.
    pub fn hide() -> Self {
        Self::new(LETTERBOX_INSTANT_SPEED, 0.0)
    }

    /// Moves bars to a target using default speed.
    pub fn move_to(target: f32) -> Self {
        Self::new(LETTERBOX_NORMAL_SPEED, target)
    }

    /// Moves bars to a target using the provided speed.
    pub fn move_to_at(target: f32, speed: f32) -> Self {
        Self::new(speed, target)
    }
}

#[derive(Clone, Debug, Deserialize, Reflect, Serialize)]
/// Serializable command used inside cutscenes.
pub enum LetterboxMove {
    To(f32),
    ToAt(f32, f32),
    Hide,
    Show,
    Close,
    Open,
}

impl From<LetterboxMove> for LetterboxMoveTrigger {
    fn from(x: LetterboxMove) -> Self {
        match x {
            LetterboxMove::To(target) => LetterboxMoveTrigger::move_to(target),
            LetterboxMove::ToAt(target, speed) => LetterboxMoveTrigger::move_to_at(target, speed),
            LetterboxMove::Hide => LetterboxMoveTrigger::hide(),
            LetterboxMove::Show => LetterboxMoveTrigger::show(),
            LetterboxMove::Close => LetterboxMoveTrigger::close(),
            LetterboxMove::Open => LetterboxMoveTrigger::open(),
        }
    }
}

impl From<LetterboxMoveTrigger> for LetterboxMove {
    fn from(e: LetterboxMoveTrigger) -> Self {
        let LetterboxMoveTrigger { target, speed } = e;
        LetterboxMove::ToAt(target, speed)
    }
}
