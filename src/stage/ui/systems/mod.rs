pub mod state;

use super::components::ScoreText;
use crate::stage::score::components::Score;
use bevy::prelude::*;
use seldom_pixel::prelude::PxText;

pub fn update_score_text(mut query: Query<&mut PxText, With<ScoreText>>, score: Res<Score>) {
    for mut text in query.iter_mut() {
        text.0 = score.value.to_string();
    }
}
