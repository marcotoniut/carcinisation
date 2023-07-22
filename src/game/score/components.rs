use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct Score {
    pub value: u32,
}

#[derive(Resource)]
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
