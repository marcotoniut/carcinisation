use super::spawn::*;
use crate::components::VolumeSettings;
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
    trigger: Trigger<StageStartupTrigger>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<StagePluginUpdateState>>,
    asset_server: Res<AssetServer>,
    volume_settings: Res<VolumeSettings>,
) {
    let data = trigger.event().data.as_ref();
    next_state.set(StagePluginUpdateState::Active);

    commands.insert_resource::<StageData>(data.clone());

    for spawn in &data.spawns {
        spawn_hud(&mut commands, &asset_server);

        #[cfg(debug_assertions)]
        info!("Spawning {:?}", spawn.show_type());

        match spawn {
            StageSpawn::Destructible(spawn) => {
                spawn_destructible(&mut commands, &asset_server, spawn);
            }
            StageSpawn::Enemy(spawn) => {
                spawn_enemy(&mut commands, Vec2::ZERO, spawn);
            }
            StageSpawn::Object(spawn) => {
                spawn_object(&mut commands, &asset_server, spawn);
            }
            StageSpawn::Pickup(spawn) => {
                spawn_pickup(&mut commands, &asset_server, Vec2::ZERO, spawn);
            }
        }
    }

    commands
        .spawn((Stage, Name::new("Stage")))
        .with_children(|p0| {
            p0.spawn(BackgroundBundle::new(PxSprite(
                asset_server.load(data.background_path.clone()),
            )));
            p0.spawn(SkyboxBundle::new(&asset_server, data.skybox.clone()));
        });

    // DEBUG

    // TODO turn this into a spawn, like in cutscene, or make it a StageSpawn
    let music_bundle = make_music_bundle(
        &asset_server,
        &volume_settings,
        data.music_path.clone(),
        PlaybackMode::Loop,
    );

    commands.spawn((music_bundle, StageEntity));

    commands.trigger(PlayerStartupTrigger);
}
