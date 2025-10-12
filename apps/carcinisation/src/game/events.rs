//! Game lifecycle events (startup, game over).

use bevy::prelude::*;

#[derive(Clone, Event)]
/// Raised when the player loses all lives or triggers game over.
pub struct GameOverTrigger {
    /// TODO review score
    pub score: u32,
}

#[derive(Event)]
/// Starts the gameplay loop.
pub struct GameStartupTrigger;
