use bevy::prelude::*;

use super::components::*;
use crate::game::events::GameOverTrigger;

pub fn on_game_over_update_high_scores(
    mut reader: EventReader<GameOverTrigger>,
    mut high_scores: ResMut<HighScores>,
) {
    for game_over in reader.read() {
        high_scores
            .scores
            .push(("Player".to_string(), game_over.score));
        high_scores.scores.sort();
        high_scores.scores.reverse();
        high_scores.scores.truncate(5);
    }
}

pub fn debug_high_scores_updated(high_scores: Res<HighScores>) {
    if high_scores.is_changed() {
        info!("High scores updated!");
        for (i, (name, score)) in high_scores.scores.iter().enumerate() {
            println!("{}. {} - {}", i + 1, name, score);
        }
    }
}
