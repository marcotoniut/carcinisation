use bevy::prelude::*;

use super::super::components::*;

pub fn spawn_cutscene(mut commands: Commands, asset_server: Res<AssetServer>) {
    build_screen(&mut commands, &asset_server);
}

pub fn despawn_cutscene(mut commands: Commands, query: Query<Entity, With<Cutscene>>) {
    if let Ok(main_menu_entity) = query.get_single() {
        commands.entity(main_menu_entity).despawn_recursive();
    }
}

pub fn build_screen(commands: &mut Commands, asset_server: &Res<AssetServer>) -> Entity {
    let main_menu_entity = commands.spawn((Cutscene {},)).id();

    return main_menu_entity;
}
