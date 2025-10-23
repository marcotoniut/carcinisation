use crate::pixel::PxAssets;
use crate::{
    debug::plugin::{debug_print_shutdown, debug_print_startup},
    globals::mark_for_despawn_by_query,
    stage::player::{
        bundles::make_player_bundle,
        components::Player,
        crosshair::CrosshairSettings,
        events::{PlayerShutdownTrigger, PlayerStartupTrigger},
    },
};
use bevy::prelude::*;
use seldom_pixel::prelude::PxSprite;

const DEBUG_MODULE: &str = "Player";

pub fn on_player_startup(
    _trigger: On<PlayerStartupTrigger>,
    mut commands: Commands,
    mut assets_sprite: PxAssets<PxSprite>,
    crosshair_settings: Res<CrosshairSettings>,
) {
    #[cfg(debug_assertions)]
    debug_print_startup(DEBUG_MODULE);

    commands.spawn(make_player_bundle(&mut assets_sprite, &crosshair_settings));
}

pub fn on_player_shutdown(
    _trigger: On<PlayerShutdownTrigger>,
    mut commands: Commands,
    query: Query<Entity, With<Player>>,
) {
    #[cfg(debug_assertions)]
    debug_print_shutdown(DEBUG_MODULE);

    mark_for_despawn_by_query(&mut commands, &query);
}
