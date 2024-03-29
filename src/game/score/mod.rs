pub mod components;
pub mod systems;

use bevy::prelude::*;

use self::{components::*, systems::*};

pub struct ScorePlugin;

impl Plugin for ScorePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HighScores>()
            .init_resource::<Score>()
            .add_systems(Update, (update_high_scores, high_scores_updated));
    }
}
