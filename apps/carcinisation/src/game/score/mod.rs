//! Score tracking plugin and systems.

pub mod components;
mod systems;

use bevy::prelude::*;

use self::{components::*, systems::*};

/// Maintains run score and high-score tables.
pub struct ScorePlugin;

impl Plugin for ScorePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HighScores>()
            .init_resource::<Score>()
            .add_systems(Update, on_game_over_update_high_scores);
        #[cfg(debug_assertions)]
        {
            app.add_systems(Update, debug_high_scores_updated);
        }
    }
}
