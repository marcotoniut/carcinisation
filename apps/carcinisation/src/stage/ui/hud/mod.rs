pub mod components;
pub mod spawn;

use crate::stage::{
    components::interactive::Health, player::components::Player, ui::hud::components::HealthText,
};
use activable::{Activable, ActivableAppExt};
use bevy::prelude::*;
use seldom_pixel::prelude::PxText;

// TODO Review separator comments

#[derive(Activable)]
pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_active_systems::<HudPlugin, _>(update_health_text);
    }
}

pub fn update_health_text(
    mut query: Query<&mut PxText, With<HealthText>>,
    player_query: Query<&Health, With<Player>>,
) {
    for mut text in query.iter_mut() {
        match player_query.single() {
            Ok(health) => {
                text.value = health.0.to_string();
            }
            Err(_) => return,
        };
    }
}
