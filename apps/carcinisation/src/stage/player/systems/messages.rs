use crate::assets::CxAssets;
#[cfg(debug_assertions)]
use crate::debug::plugin::{debug_print_shutdown, debug_print_startup};
use crate::{
    globals::mark_for_despawn_by_query,
    stage::player::{
        bundles::make_player_bundle,
        components::Player,
        crosshair::CrosshairSettings,
        messages::{PlayerShutdownEvent, PlayerStartupEvent},
    },
};
use bevy::prelude::*;
use carapace::prelude::CxSprite;

const DEBUG_MODULE: &str = "Player";

/// @trigger Spawns the player entity and crosshair on `PlayerStartupEvent`.
pub fn on_player_startup(
    _trigger: On<PlayerStartupEvent>,
    mut commands: Commands,
    mut assets_sprite: CxAssets<CxSprite>,
    crosshair_settings: Res<CrosshairSettings>,
) {
    #[cfg(debug_assertions)]
    debug_print_startup(DEBUG_MODULE);

    commands.spawn(make_player_bundle(&mut assets_sprite, &crosshair_settings));
}

/// @trigger Despawns all player entities on `PlayerShutdownEvent`.
pub fn on_player_shutdown(
    _trigger: On<PlayerShutdownEvent>,
    mut commands: Commands,
    query: Query<Entity, With<Player>>,
) {
    #[cfg(debug_assertions)]
    debug_print_shutdown(DEBUG_MODULE);

    mark_for_despawn_by_query(&mut commands, &query);
}
