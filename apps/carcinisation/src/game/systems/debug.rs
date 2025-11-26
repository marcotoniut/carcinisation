//! Debug-only logging for game systems.

use bevy::prelude::*;

use crate::game::messages::GameOverEvent;

/// @trigger Logs the player's score when the game ends (debug builds).
pub fn debug_on_game_over(trigger: On<GameOverEvent>) {
    let e = trigger.event();
    info!("Your final score: {}", e.score);
}
