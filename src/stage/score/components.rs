use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct Score {
    pub value: u32,
}

impl Score {
    pub fn add(&mut self, value: i32) {
        let score: i32 = self.value as i32 + value;
        if score < 0 {
            self.value = 0;
        } else {
            self.value = score as u32;
        }
    }

    pub fn add_u(&mut self, value: u32) {
        self.value += value;
    }
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
