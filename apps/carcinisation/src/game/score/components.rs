//! Score-related resources.

use bevy::prelude::*;
pub use carcinisation_base::game::Score;

#[derive(Resource)]
/// Stores the top high scores for display.
pub struct HighScores {
    pub scores: Vec<(String, u32)>,
}

impl Default for HighScores {
    fn default() -> Self {
        HighScores {
            scores: vec![
                ("Player 1".to_string(), 100),
                ("Player 2".to_string(), 80),
                ("Player 3".to_string(), 60),
                ("Player 4".to_string(), 40),
                ("Player 5".to_string(), 20),
            ],
        }
    }
}
