use crate::stage::{
    components::interactive::Health, player::components::Player, ui::hud::components::HealthText,
};
use bevy::prelude::*;
use seldom_pixel::prelude::PxText;

pub fn update_health_text(
    mut query: Query<&mut PxText, With<HealthText>>,
    player_query: Query<&Health, With<Player>>,
) {
    for health in player_query.iter() {
        for mut text in query.iter_mut() {
            text.value = health.0.to_string();
        }
    }
}
