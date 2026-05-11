//! Shared game state types used across game modes.

use bevy::prelude::*;

/// Coarse game states used to drive menus/cutscenes/stage logic.
#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
pub enum GameProgressState {
    #[default]
    Loading,
    Running,
    Paused,
    Cutscene,
}

/// Tracks the current run score.
#[derive(Resource, Default)]
pub struct Score {
    pub value: u32,
}

impl Score {
    /// Add a signed value (negative = penalty).
    pub fn add(&mut self, value: i32) {
        let abs = value.unsigned_abs();
        if value >= 0 {
            self.value = self.value.saturating_add(abs);
        } else {
            self.value = self.value.saturating_sub(abs);
        }
    }

    /// Add an unsigned value.
    pub fn add_u(&mut self, value: u32) {
        self.value += value;
    }
}

/// Remaining player lives.
#[derive(Resource)]
pub struct Lives(pub u8);

/// Tracks which game step is currently active.
#[derive(Resource, Default, Clone, Copy)]
pub struct GameProgress {
    pub index: usize,
}

/// Raised when the player loses all lives — signals end of run.
#[derive(Clone, Event, Message)]
pub struct GameOverEvent {
    pub score: u32,
}

/// Number of lives the player starts a run with.
pub const STARTING_LIVES: u8 = 3;

/// Score penalty applied on each player death.
pub const DEATH_SCORE_PENALTY: i32 = 150;

/// Marker component for the camera entity.
#[derive(Component)]
pub struct CameraPos;
