use super::components::ScoreText;
use crate::game::score::components::Score;
use bevy::prelude::*;
use carapace::prelude::PxText;

/// @system Refreshes the on-screen score display when the score changes.
pub fn update_score_text(mut query: Query<&mut PxText, With<ScoreText>>, score: Res<Score>) {
    for mut text in &mut query {
        text.value = score.value.to_string();
    }
}
