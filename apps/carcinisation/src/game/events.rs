//! Game lifecycle events (startup, game over).

use bevy::prelude::*;

#[derive(Clone, Event, Message)]
/// Raised when the player loses all lives or triggers game over.
pub struct GameOverTrigger {
    /// TODO review score
    pub score: u32,
}

#[derive(Event, Message)]
/// Starts the gameplay loop.
pub struct GameStartupTrigger;
