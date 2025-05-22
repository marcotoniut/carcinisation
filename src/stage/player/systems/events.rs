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
use seldom_pixel::prelude::*;

const DEBUG_MODULE: &str = "Player";

pub fn on_player_startup(
    _trigger: Trigger<PlayerStartupTrigger>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    crosshair_settings: Res<CrosshairSettings>,
) {
    #[cfg(debug_assertions)]
    debug_print_startup(DEBUG_MODULE);

    commands.spawn(make_player_bundle(&asset_server, &crosshair_settings));
}

pub fn on_player_shutdown(
    _trigger: Trigger<PlayerShutdownTrigger>,
    mut commands: Commands,
    query: Query<Entity, With<Player>>,
) {
    #[cfg(debug_assertions)]
    debug_print_shutdown(DEBUG_MODULE);

    mark_for_despawn_by_query(&mut commands, &query);
}
