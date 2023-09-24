use bevy::prelude::*;

use super::components::*;
use crate::game::events::GameOver;

pub fn insert_score(mut commands: Commands) {
    commands.insert_resource(Score::default());
}

pub fn remove_score(mut commands: Commands) {
    commands.remove_resource::<Score>();
}

pub fn update_score(score: Res<Score>) {
    if score.is_changed() {
        info!("Score: {}", score.value.to_string())
    }
}

pub fn update_high_scores(
    mut game_over_event_reader: EventReader<GameOver>,
    mut high_scores: ResMut<HighScores>,
) {
    for game_over in game_over_event_reader.iter() {
        high_scores
            .scores
            .push(("Player".to_string(), game_over.score));
        high_scores.scores.sort();
        high_scores.scores.reverse();
        high_scores.scores.truncate(5);
    }
}

pub fn high_scores_updated(high_scores: Res<HighScores>) {
    if high_scores.is_changed() {
        info!("High scores updated!");
        for (i, (name, score)) in high_scores.scores.iter().enumerate() {
            info!("{}. {} - {}", i + 1, name, score);
        }
    }
}
