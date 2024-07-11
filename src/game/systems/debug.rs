use bevy::prelude::*;

use crate::game::GameOverTrigger;

pub fn debug_on_game_over(trigger: Trigger<GameOverTrigger>) {
    let e = trigger.event();
    info!("Your final score: {}", e.score);
}
