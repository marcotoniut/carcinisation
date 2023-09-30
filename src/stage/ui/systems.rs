use bevy::prelude::*;
use seldom_pixel::prelude::PxText;

use crate::stage::score::components::Score;

use super::components::ScoreText;

pub fn update_score_text(mut query: Query<&mut PxText, With<ScoreText>>, score: Res<Score>) {
    for mut text in query.iter_mut() {
        text.0 = score.value.to_string();
    }
}
