use crate::{
    resource::debug::STAGE_DEBUG_DATA,
    stage::{
        data::{StageData, StageSpawn},
        player::events::PlayerStartupEvent,
    },
    systems::{audio::VolumeSettings, spawn::spawn_music},
};
use bevy::{audio::PlaybackMode, prelude::*};
use seldom_pixel::{prelude::PxAssets, sprite::PxSprite};

use super::spawn::*;

pub fn insert_stage_resource(mut commands: Commands) {
    // let stage_data = STAGE_PARK_DATA.clone();
    let stage_data = STAGE_DEBUG_DATA.clone();
    commands.insert_resource::<StageData>(stage_data);
}

pub fn setup_stage(
    mut commands: Commands,
    mut assets_sprite: PxAssets<PxSprite>,
    mut event_writer: EventWriter<PlayerStartupEvent>,
    asset_server: Res<AssetServer>,
    stage_data: Res<StageData>,
    volume_settings: Res<VolumeSettings>,
) {
    event_writer.send(PlayerStartupEvent);

    for spawn in &stage_data.spawns {
        match spawn {
            StageSpawn::Destructible(spawn) => {
                spawn_destructible(&mut commands, &mut assets_sprite, spawn);
            }
            StageSpawn::Enemy(spawn) => {
                spawn_enemy(&mut commands, Vec2::ZERO, spawn);
            }
            StageSpawn::Object(spawn) => {
                spawn_object(&mut commands, &mut assets_sprite, spawn);
            }
            StageSpawn::Pickup(spawn) => {
                spawn_pickup(&mut commands, &mut assets_sprite, Vec2::ZERO, spawn);
            }
        }
    }

    spawn_music(
        &mut commands,
        &asset_server,
        &volume_settings,
        stage_data.music_path.clone(),
        PlaybackMode::Loop,
    );
}
