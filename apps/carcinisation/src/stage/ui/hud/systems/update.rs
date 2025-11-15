use super::super::spawn::{
    HUD_HEALTH_ICON_WIDTH, HUD_HEALTH_ICON_X, HUD_HEALTH_LAYOUT_Y, HUD_HEALTH_TEXT_CHAR_WIDTH,
    HUD_HEALTH_TEXT_PADDING,
};
use crate::stage::{
    components::interactive::Health, player::components::Player, ui::hud::components::HealthText,
};
use bevy::prelude::*;
use seldom_pixel::prelude::{PxPosition, PxText};

pub fn update_health_text(
    mut query: Query<(&mut PxText, &mut PxPosition), With<HealthText>>,
    player_query: Query<&Health, With<Player>>,
) {
    let health = match player_query.single() {
        Ok(health) => health,
        Err(_) => return,
    };

    let value = health.0.to_string();
    let digit_count = value.len() as i32;
    let text_width = digit_count * HUD_HEALTH_TEXT_CHAR_WIDTH;
    let right_x =
        HUD_HEALTH_ICON_X as i32 + HUD_HEALTH_ICON_WIDTH + HUD_HEALTH_TEXT_PADDING + text_width;

    for (mut text, mut position) in query.iter_mut() {
        text.value = value.clone();
        position.x = right_x;
        position.y = HUD_HEALTH_LAYOUT_Y;
    }
}
