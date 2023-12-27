use crate::{
    components::DespawnMark,
    stage::player::{
        bundles::make_player_bundle,
        components::Player,
        crosshair::CrosshairSettings,
        events::{PlayerShutdownEvent, PlayerStartupEvent},
    },
};
use bevy::prelude::*;
use seldom_pixel::prelude::*;

pub fn on_startup(
    mut commands: Commands,
    mut assets_sprite: PxAssets<PxSprite>,
    mut event_reader: EventReader<PlayerStartupEvent>,
    crosshair_settings: Res<CrosshairSettings>,
) {
    for _ in event_reader.read() {
        commands.spawn(make_player_bundle(&mut assets_sprite, &crosshair_settings));
    }
}

pub fn on_shutdown(
    mut commands: Commands,
    mut event_reader: EventReader<PlayerShutdownEvent>,
    query: Query<Entity, With<Player>>,
) {
    for _ in event_reader.read() {
        for entity in query.iter() {
            commands.entity(entity).insert(DespawnMark);
        }
    }
}
