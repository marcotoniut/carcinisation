use super::spawn::*;
use crate::{
    stage::{
        bundles::{make_background_bundle, make_skybox_bundle},
        components::{Stage, StageEntity},
        data::{StageData, StageSpawn},
        events::StageStartupEvent,
        player::events::PlayerStartupEvent,
        ui::hud::spawn::spawn_hud,
        StagePluginUpdateState,
    },
    systems::{audio::VolumeSettings, spawn::make_music_bundle},
};
use bevy::{audio::PlaybackMode, prelude::*};
use seldom_pixel::{
    prelude::{PxAssets, PxFilter, PxTypeface},
    sprite::PxSprite,
};

pub fn on_startup(
    mut commands: Commands,
    mut filters: PxAssets<PxFilter>,
    mut assets_sprite: PxAssets<PxSprite>,
    mut typefaces: PxAssets<PxTypeface>,
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
            spawn_hud(
                &mut commands,
                &mut typefaces,
                &mut assets_sprite,
                &mut filters,
            );

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

        // DEBUG

        // TODO turn this into a spawn, like in cutscene, or make it a StageSpawn
        // let music_bundle = make_music_bundle(
        //     &asset_server,
        //     &volume_settings,
        //     e.data.music_path.clone(),
        //     PlaybackMode::Loop,
        // );

        // commands.spawn((music_bundle, StageEntity));
    }
}
