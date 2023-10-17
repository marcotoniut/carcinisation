pub mod setup;

use super::{
    data::DEATH_SCORE_PENALTY, events::GameOverEvent, resources::Lives, score::components::Score,
    GamePluginUpdateState,
};
use crate::stage::{components::interactive::Dead, player::components::Player};
use bevy::prelude::*;

pub fn pause_game(mut game_state_next_state: ResMut<NextState<GamePluginUpdateState>>) {
    game_state_next_state.set(GamePluginUpdateState::Active);
}

pub fn resume_game(mut game_state_next_state: ResMut<NextState<GamePluginUpdateState>>) {
    game_state_next_state.set(GamePluginUpdateState::Inactive);
}

pub fn check_player_died(
    mut score: ResMut<Score>,
    mut query: Query<(Added<Dead>, With<Player>)>,
    mut event_writer: EventWriter<GameOverEvent>,
    mut lives: ResMut<Lives>,
) {
    if let Ok(_) = query.get_single_mut() {
        score.add(-DEATH_SCORE_PENALTY);
        lives.0 = lives.0.saturating_sub(1).max(0);
        if lives.0 == 0 {
            event_writer.send(GameOverEvent { score: score.value });
        } else {
            // TODO restart from checkpoint
        }
    }
}
