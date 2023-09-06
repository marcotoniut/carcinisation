use bevy::prelude::*;

use super::super::components::*;

pub fn spawn_main_menu(mut commands: Commands) {
    build_screen(&mut commands);
}

pub fn despawn_main_menu(mut commands: Commands, query: Query<Entity, With<MainMenu>>) {
    if let Ok(entity) = query.get_single() {
        commands.entity(entity).despawn_recursive();
    }
}

pub fn build_screen(commands: &mut Commands) -> Entity {
    let entity = commands.spawn((MainMenu {},)).id();

    return entity;
}
