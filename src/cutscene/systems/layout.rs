use bevy::prelude::*;
use seldom_pixel::{asset::*, filter::*};

use crate::{
    cutscene::bundles::make_letterbox_row,
    globals::{mark_for_despawn_by_component_query, SCREEN_RESOLUTION},
};

use super::super::components::*;

pub fn spawn_cutscene(mut commands: Commands, mut filters: PxAssets<PxFilter>) {
    build_screen(&mut commands, &mut filters);
}

pub fn mark_cutscene_for_despawn(mut commands: Commands, query: Query<Entity, With<Cutscene>>) {
    mark_for_despawn_by_component_query(&mut commands, &query);
}

pub fn build_letterbox_top(
    commands: &mut ChildBuilder<'_, '_, '_>,
    filter: Handle<PxAsset<PxFilterData>>,
) {
    for row in 0..LETTERBOX_HEIGHT {
        commands.spawn(make_letterbox_row(filter.clone(), row));
    }
}

pub fn build_letterbox_bottom(
    commands: &mut ChildBuilder<'_, '_, '_>,
    filter: Handle<PxAsset<PxFilterData>>,
) {
    for row in (SCREEN_RESOLUTION.y - LETTERBOX_HEIGHT)..SCREEN_RESOLUTION.y {
        commands.spawn(make_letterbox_row(filter.clone(), row));
    }
}

pub fn build_screen(commands: &mut Commands, filters: &mut PxAssets<PxFilter>) -> Entity {
    let letterbox_filter = filters.load("filter/color1.png");

    let mut entity_commands = commands.spawn(Cutscene {});

    entity_commands.with_children(|parent| {
        build_letterbox_top(parent, letterbox_filter.clone());
        build_letterbox_bottom(parent, letterbox_filter.clone());
    });

    return entity_commands.id();
}
