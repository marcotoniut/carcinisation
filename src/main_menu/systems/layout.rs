use bevy::prelude::*;

use crate::globals::mark_for_despawn_by_component_query;

use super::super::components::*;

pub fn spawn_main_menu(mut commands: Commands) {
    build_screen(&mut commands);
}

pub fn despawn_main_menu(mut commands: Commands, query: Query<Entity, With<MainMenu>>) {
    mark_for_despawn_by_component_query(&mut commands, &query)
}

pub fn build_screen(commands: &mut Commands) -> Entity {
    let entity = commands.spawn((MainMenu {},)).id();

    return entity;
}
