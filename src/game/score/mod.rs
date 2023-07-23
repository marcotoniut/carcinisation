use bevy::prelude::*;

use crate::AppState;

use self::{components::*, systems::*};

pub mod components;
pub mod resources;
pub mod systems;

pub struct ScorePlugin;

impl Plugin for ScorePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HighScores>()
            .add_systems(OnEnter(AppState::Game), insert_score)
            .add_systems(Startup, update_score.run_if(in_state(AppState::Game)))
            .add_systems(Update, update_high_scores)
            .add_systems(Update, high_scores_updated)
            .add_systems(OnExit(AppState::Game), remove_score);
    }
}
