use bevy::prelude::*;
use seldom_pixel::prelude::PxText;

use crate::stage::{
    components::interactive::Health, player::components::Player, ui::hud::components::HealthText,
};

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
