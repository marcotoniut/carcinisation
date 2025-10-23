use super::spawn::*;
use crate::components::VolumeSettings;
use crate::pixel::PxAssets;
use crate::{
    stage::{
        bundles::{BackgroundBundle, SkyboxBundle},
        components::{Stage, StageEntity},
        data::{StageData, StageSpawn},
        events::StageStartupTrigger,
        player::events::PlayerStartupTrigger,
        ui::hud::spawn::spawn_hud,
        StagePluginUpdateState,
    },
    systems::spawn::make_music_bundle,
};
use bevy::{audio::PlaybackMode, prelude::*};
use seldom_pixel::prelude::{PxFilter, PxSprite, PxTypeface};

pub fn on_stage_startup(
    trigger: On<StageStartupTrigger>,
    mut commands: Commands,
    mut filters: PxAssets<PxFilter>,
    mut assets_sprite: PxAssets<PxSprite>,
    mut typefaces: PxAssets<PxTypeface>,
    mut next_state: ResMut<NextState<StagePluginUpdateState>>,
    asset_server: Res<AssetServer>,
    volume_settings: Res<VolumeSettings>,
) {
    let data = trigger.event().data.as_ref();
    next_state.set(StagePluginUpdateState::Active);

    commands.insert_resource::<StageData>(data.clone());

    for spawn in &data.spawns {
        spawn_hud(
            &mut commands,
            &mut typefaces,
            &mut assets_sprite,
            &mut filters,
        );

        #[cfg(debug_assertions)]
        info!("Spawning {:?}", spawn.show_type());

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
        .spawn((Stage, Name::new("Stage"), Visibility::Visible))
        .with_children(|p0| {
            p0.spawn(BackgroundBundle::new(
                assets_sprite.load(data.background_path.clone()),
            ));
            p0.spawn(SkyboxBundle::new(&mut assets_sprite, data.skybox.clone()));
        });

    // DEBUG

    // TODO turn this into a spawn, like in cutscene, or make it a StageSpawn
    let (player, settings, system_bundle, music_tag) = make_music_bundle(
        &asset_server,
        &volume_settings,
        data.music_path.clone(),
        PlaybackMode::Loop,
    );

    commands.spawn((player, settings, system_bundle, music_tag, StageEntity));

    commands.trigger(PlayerStartupTrigger);
}
