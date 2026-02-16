//! Game lifecycle events (startup, game over).

use bevy::prelude::*;

#[derive(Clone, Event, Message)]
/// Raised when the player loses all lives or triggers game over.
pub struct GameOverEvent {
    pub score: u32,
}

#[derive(Event, Message)]
/// Starts the gameplay loop.
pub struct GameStartupEvent;
