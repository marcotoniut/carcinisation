use super::spawn::*;
use crate::{
    components::VolumeSettings,
    game::GameProgressState,
    pixel::PxAssets,
    stage::{
        StagePlugin,
        bundles::{BackgroundBundle, SkyboxBundle},
        components::{Stage, StageEntity},
        data::{StageData, StageSpawn},
        messages::StageStartupEvent,
        player::messages::PlayerStartupEvent,
        ui::hud::spawn::spawn_hud,
    },
    systems::spawn::make_music_bundle,
    transitions::trigger_transition,
};
use activable::activate;
use bevy::{audio::PlaybackMode, prelude::*};
use seldom_pixel::prelude::{PxFilter, PxSprite, PxTypeface};

/// @trigger Builds the stage world: spawns HUD, enemies, pickups, background, and music.
#[allow(clippy::too_many_arguments)]
pub fn on_stage_startup(
    trigger: On<StageStartupEvent>,
    mut commands: Commands,
    mut next_game_state: ResMut<NextState<GameProgressState>>,
    mut assets_sprite: PxAssets<PxSprite>,
    mut filters: PxAssets<PxFilter>,
    mut typefaces: PxAssets<PxTypeface>,
    asset_server: Res<AssetServer>,
    volume_settings: Res<VolumeSettings>,
) {
    let data = trigger.event().data.as_ref();
    activate::<StagePlugin>(&mut commands);

    commands.insert_resource::<StageData>(data.clone());

    if let Some(request) = &data.on_start_transition_o {
        trigger_transition(&mut commands, request);
    }

    spawn_hud(
        &mut commands,
        &mut typefaces,
        &mut assets_sprite,
        &mut filters,
    );

    for spawn in &data.spawns {
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

    next_game_state.set(GameProgressState::Running);
    commands.trigger(PlayerStartupEvent);
}
