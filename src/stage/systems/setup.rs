use crate::{
    resource::debug::STAGE_DEBUG_DATA,
    stage::{
        bundles::{make_background_bundle, make_skybox_bundle},
        components::Stage,
        data::{StageData, StageSpawn},
        events::StageStartupEvent,
        player::events::PlayerStartupEvent,
        StagePluginUpdateState,
    },
    systems::{audio::VolumeSettings, spawn::spawn_music},
};
use bevy::{audio::PlaybackMode, prelude::*};
use seldom_pixel::{prelude::PxAssets, sprite::PxSprite};

use super::spawn::*;

pub fn on_startup(
    mut commands: Commands,
    mut assets_sprite: PxAssets<PxSprite>,
    mut event_writer: EventWriter<PlayerStartupEvent>,
    mut event_reader: EventReader<StageStartupEvent>,
    mut stage_state_next_state: ResMut<NextState<StagePluginUpdateState>>,
    asset_server: Res<AssetServer>,

    volume_settings: Res<VolumeSettings>,
) {
    for e in event_reader.iter() {
        stage_state_next_state.set(StagePluginUpdateState::Active);

        event_writer.send(PlayerStartupEvent);
        let stage_data = e.data.as_ref();

        commands.insert_resource::<StageData>(stage_data.clone());

        for spawn in &e.data.spawns {
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

        commands
            .spawn((Stage, Name::new("Stage")))
            .with_children(|parent| {
                let background_bundle =
                    make_background_bundle(&mut assets_sprite, stage_data.background_path.clone());
                parent.spawn(background_bundle);

                let skybox_bundle =
                    make_skybox_bundle(&mut assets_sprite, stage_data.skybox.clone());
                parent.spawn(skybox_bundle);
            });

        spawn_music(
            &mut commands,
            &asset_server,
            &volume_settings,
            e.data.music_path.clone(),
            PlaybackMode::Loop,
        );
    }
}
