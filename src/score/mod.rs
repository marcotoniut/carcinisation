use bevy::prelude::*;

use self::{components::*, systems::*};

pub mod components;
pub mod resources;
pub mod systems;

pub struct ScorePlugin;

impl Plugin for ScorePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Score>()
            .init_resource::<HighScores>()
            .add_systems(Startup, update_score)
            .add_systems(Update, update_high_scores)
            .add_systems(Update, high_scores_updated);
    }
}
