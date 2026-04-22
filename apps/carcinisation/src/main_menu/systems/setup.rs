//! Startup/shutdown transitions for the main menu.

#[cfg(debug_assertions)]
use crate::debug::plugin::{debug_print_shutdown, debug_print_startup};
use crate::{
    components::VolumeSettings,
    globals::mark_for_despawn_by_query,
    main_menu::{
        MainMenuScreen,
        components::{MainMenu, MainMenuEntity, MainMenuMusic},
    },
    systems::{camera::CameraPos, spawn::make_music_bundle},
};
use assert_assets_path::assert_assets_path;
use bevy::{audio::PlaybackMode, prelude::*};
use carapace::prelude::WorldPos;

#[cfg(debug_assertions)]
const DEBUG_MODULE: &str = "MainMenu";

/// @system Enters the press-start screen when the main menu activates.
///
/// Resets the camera to the origin so the world-space menu background
/// (rendered at 0,0) is visible.  Without this, returning from a stage
/// leaves the camera at its last gameplay position and the background
/// ends up off-screen.
pub fn on_main_menu_startup(
    mut screen_state: ResMut<NextState<MainMenuScreen>>,
    mut camera_query: Query<&mut WorldPos, With<CameraPos>>,
) {
    #[cfg(debug_assertions)]
    debug_print_startup(DEBUG_MODULE);

    if let Ok(mut cam) = camera_query.single_mut() {
        cam.0 = Vec2::ZERO;
    }

    screen_state.set(MainMenuScreen::PressStart);
}

/// @system Cleans up main menu entities when the main menu deactivates.
pub fn on_main_menu_shutdown(
    mut commands: Commands,
    mut screen_state: ResMut<NextState<MainMenuScreen>>,
    main_menu_query: Query<Entity, With<MainMenu>>,
    main_menu_entity_query: Query<Entity, With<MainMenuEntity>>,
) {
    #[cfg(debug_assertions)]
    debug_print_shutdown(DEBUG_MODULE);

    screen_state.set(MainMenuScreen::Disabled);

    mark_for_despawn_by_query(&mut commands, &main_menu_query);
    mark_for_despawn_by_query(&mut commands, &main_menu_entity_query);
}

/// @system Spawns the looping main-menu music track.
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

/// @system Despawns the main-menu music entity.
pub fn cleanup_main_menu_music(
    mut commands: Commands,
    music_query: Query<Entity, With<MainMenuMusic>>,
) {
    mark_for_despawn_by_query(&mut commands, &music_query);
}
