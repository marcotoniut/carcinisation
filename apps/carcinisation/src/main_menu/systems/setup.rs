//! Startup/shutdown transitions for the main menu.

use crate::{
    components::VolumeSettings,
    debug::plugin::{debug_print_shutdown, debug_print_startup},
    globals::mark_for_despawn_by_query,
    main_menu::{
        components::{MainMenu, MainMenuEntity, MainMenuMusic},
        MainMenuPlugin, MainMenuScreen,
    },
    systems::spawn::make_music_bundle,
};
use activable::{activate, deactivate};
use assert_assets_path::assert_assets_path;
use bevy::{audio::PlaybackMode, prelude::*};

#[cfg(debug_assertions)]
const DEBUG_MODULE: &str = "MainMenu";

pub fn on_main_menu_startup(
    mut commands: Commands,
    mut screen_state: ResMut<NextState<MainMenuScreen>>,
) {
    #[cfg(debug_assertions)]
    debug_print_startup(DEBUG_MODULE);

    activate::<MainMenuPlugin>(&mut commands);
    screen_state.set(MainMenuScreen::PressStart);
}

/// @trigger Cleans up main menu entities and disables the plugin.
pub fn on_main_menu_shutdown(
    mut commands: Commands,
    mut screen_state: ResMut<NextState<MainMenuScreen>>,
    main_menu_query: Query<Entity, With<MainMenu>>,
    main_menu_entity_query: Query<Entity, With<MainMenuEntity>>,
) {
    #[cfg(debug_assertions)]
    debug_print_shutdown(DEBUG_MODULE);

    deactivate::<MainMenuPlugin>(&mut commands);
    screen_state.set(MainMenuScreen::Disabled);

    mark_for_despawn_by_query(&mut commands, &main_menu_query);
    mark_for_despawn_by_query(&mut commands, &main_menu_entity_query);
}

pub fn spawn_main_menu_music(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    volume_settings: Res<VolumeSettings>,
) {
    let (player, settings, system_bundle, music_tag) = make_music_bundle(
        &asset_server,
        &volume_settings,
        assert_assets_path!("audio/music/intro.ogg").to_string(),
        PlaybackMode::Loop,
    );

    commands.spawn((player, settings, system_bundle, music_tag, MainMenuMusic));
}

pub fn cleanup_main_menu_music(
    mut commands: Commands,
    music_query: Query<Entity, With<MainMenuMusic>>,
) {
    mark_for_despawn_by_query(&mut commands, &music_query);
}
