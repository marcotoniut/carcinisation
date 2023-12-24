use crate::{
    globals::mark_for_despawn_by_component_query,
    main_menu::{
        components::{MainMenu, MainMenuEntity},
        events::{MainMenuShutdownEvent, MainMenuStartupEvent},
        MainMenuPluginUpdateState,
    },
};
use bevy::prelude::*;

pub fn on_startup(
    mut event_reader: EventReader<MainMenuStartupEvent>,
    mut next_state: ResMut<NextState<MainMenuPluginUpdateState>>,
) {
    for _ in event_reader.read() {
        next_state.set(MainMenuPluginUpdateState::Active);
    }
}

pub fn on_shutdown(
    mut commands: Commands,
    mut event_reader: EventReader<MainMenuShutdownEvent>,
    mut next_state: ResMut<NextState<MainMenuPluginUpdateState>>,
    main_menu_query: Query<Entity, With<MainMenu>>,
    main_menu_entity_query: Query<Entity, With<MainMenuEntity>>,
) {
    for _ in event_reader.read() {
        next_state.set(MainMenuPluginUpdateState::Inactive);

        mark_for_despawn_by_component_query(&mut commands, &main_menu_query);
        mark_for_despawn_by_component_query(&mut commands, &main_menu_entity_query);
    }
}
