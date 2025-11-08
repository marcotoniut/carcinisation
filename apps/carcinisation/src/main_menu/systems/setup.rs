//! Startup/shutdown transitions for the main menu.

use crate::{
    debug::plugin::{debug_print_shutdown, debug_print_startup},
    globals::mark_for_despawn_by_query,
    main_menu::{
        components::{MainMenu, MainMenuEntity},
        events::{MainMenuShutdownEvent, MainMenuStartupEvent},
        MainMenuPluginUpdateState, MainMenuScreen,
    },
};
use bevy::prelude::*;

#[cfg(debug_assertions)]
const DEBUG_MODULE: &str = "MainMenu";

pub fn on_main_menu_startup(
    _trigger: On<MainMenuStartupEvent>,
    mut plugin_state: ResMut<NextState<MainMenuPluginUpdateState>>,
    mut screen_state: ResMut<NextState<MainMenuScreen>>,
) {
    #[cfg(debug_assertions)]
    debug_print_startup(DEBUG_MODULE);

    plugin_state.set(MainMenuPluginUpdateState::Active);
    screen_state.set(MainMenuScreen::PressStart);
}

/// @trigger Cleans up main menu entities and disables the plugin.
pub fn on_main_menu_shutdown(
    _trigger: On<MainMenuShutdownEvent>,
    mut commands: Commands,
    mut plugin_state: ResMut<NextState<MainMenuPluginUpdateState>>,
    mut screen_state: ResMut<NextState<MainMenuScreen>>,
    main_menu_query: Query<Entity, With<MainMenu>>,
    main_menu_entity_query: Query<Entity, With<MainMenuEntity>>,
) {
    #[cfg(debug_assertions)]
    debug_print_shutdown(DEBUG_MODULE);

    plugin_state.set(MainMenuPluginUpdateState::Inactive);
    screen_state.set(MainMenuScreen::Disabled);

    mark_for_despawn_by_query(&mut commands, &main_menu_query);
    mark_for_despawn_by_query(&mut commands, &main_menu_entity_query);
}
