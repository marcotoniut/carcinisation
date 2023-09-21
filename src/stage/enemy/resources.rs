use bevy::prelude::*;

use super::components::PLACEHOLDER_ENEMY_SPAWN_TIME;

#[derive(Resource)]
pub struct EnemySpawnTimer {
    pub timer: Timer,
}

impl Default for EnemySpawnTimer {
    fn default() -> Self {
        EnemySpawnTimer {
            timer: Timer::from_seconds(PLACEHOLDER_ENEMY_SPAWN_TIME, TimerMode::Repeating),
        }
    }
}
