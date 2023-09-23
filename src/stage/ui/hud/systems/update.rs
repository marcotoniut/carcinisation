use bevy::prelude::*;
use seldom_pixel::prelude::PxText;

use crate::stage::{
    components::Health,
    enemy::components::PlaceholderEnemy,
    player::components::Player,
    ui::hud::components::{EnemyCountText, HealthText},
};

pub fn update_enemy_text(
    mut text_query: Query<&mut PxText, With<EnemyCountText>>,
    enemy_query: Query<With<PlaceholderEnemy>>,
) {
    let count = enemy_query.iter().count().to_string();
    for mut text in text_query.iter_mut() {
        text.0 = count.clone();
    }
}

pub fn update_health_text(
    mut query: Query<&mut PxText, With<HealthText>>,
    player_query: Query<&Health, With<Player>>,
) {
    for health in player_query.iter() {
        for mut text in query.iter_mut() {
            text.0 = health.0.to_string();
        }
    }
}
