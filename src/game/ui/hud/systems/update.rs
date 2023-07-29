use bevy::prelude::*;

use crate::game::{enemy::components::Enemy, score::components::*, ui::hud::components::*};

pub fn update_enemy_text(
    mut text_query: Query<&mut Text, With<EnemyText>>,
    enemy_query: Query<With<Enemy>>,
) {
    let count = enemy_query.iter().count().to_string();
    for mut text in text_query.iter_mut() {
        text.sections[0].value = count.clone();
    }
}

pub fn update_score_text(mut query: Query<&mut Text, With<ScoreText>>, score: Res<Score>) {
    if score.is_changed() {
        for mut text in query.iter_mut() {
            text.sections[0].value = score.value.to_string();
        }
    }
}
