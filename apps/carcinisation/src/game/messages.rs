//! Game lifecycle events (startup, game over).

use bevy::prelude::*;

pub use carcinisation_base::game::GameOverEvent;

#[derive(Event, Message)]
/// Starts the gameplay loop.
pub struct GameStartupEvent;
