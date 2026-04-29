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
        let score: i32 = self.value as i32 + value;
        if score < 0 {
            self.value = 0;
        } else {
            self.value = score as u32;
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

/// Marker component for the camera entity.
#[derive(Component)]
pub struct CameraPos;
