pub mod components;
pub mod systems;

use self::{components::*, systems::*};
use crate::AppState;
use bevy::prelude::*;

pub struct ScorePlugin;

impl Plugin for ScorePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HighScores>()
            .add_systems(OnEnter(AppState::Game), insert_score)
            .add_systems(
                Update,
                (update_high_scores, high_scores_updated).run_if(in_state(AppState::Game)),
            )
            .add_systems(OnExit(AppState::Game), remove_score);
    }
}
