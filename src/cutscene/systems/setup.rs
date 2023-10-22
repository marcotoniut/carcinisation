use crate::{
    components::{DespawnMark, StepStarted, Tag},
    cutscene::{
        components::{Cinematic, CutsceneEntity},
        data::{CinematicData, CutsceneAnimationSpawn},
        events::{CinematicStartupEvent, CutsceneShutdownEvent},
        resources::CutsceneProgress,
        CutscenePluginUpdateState,
    },
    globals::mark_for_despawn_by_component_query,
    systems::{audio::VolumeSettings, spawn::make_music_bundle},
    Layer,
};
use bevy::{audio::PlaybackMode, prelude::*};
use seldom_pixel::{
    prelude::{
        PxAnchor, PxAnimationBundle, PxAnimationDuration, PxAnimationFinishBehavior, PxAssets,
        PxSubPosition,
    },
    sprite::{PxSprite, PxSpriteBundle},
};

pub fn on_startup(
    mut commands: Commands,
    mut event_reader: EventReader<CinematicStartupEvent>,
    mut cutscene_state_next_state: ResMut<NextState<CutscenePluginUpdateState>>,
    asset_server: Res<AssetServer>,
    volume_settings: Res<VolumeSettings>,
) {
    for e in event_reader.iter() {
        cutscene_state_next_state.set(CutscenePluginUpdateState::Active);

        let data = e.data.as_ref();

        commands.insert_resource::<CinematicData>(data.clone());
        commands.insert_resource::<CutsceneProgress>(CutsceneProgress { index: 0 });

        commands.spawn((Cinematic, Name::new("Cutscene")));

        if let Some(music_path) = &data.music_path_o {
            let music_bundle = make_music_bundle(
                &asset_server,
                &volume_settings,
                music_path.to_string(),
                PlaybackMode::Loop,
            );

            commands.spawn((CutsceneEntity, music_bundle, Name::new("Cutscene music")));
        }
    }
}

pub fn on_shutdown(
    mut commands: Commands,
    mut event_reader: EventReader<CutsceneShutdownEvent>,
    cinematic_query: Query<Entity, With<Cinematic>>,
    cutscene_entity_query: Query<Entity, With<CutsceneEntity>>,
) {
    for _ in event_reader.iter() {
        mark_for_despawn_by_component_query(&mut commands, &cinematic_query);
        mark_for_despawn_by_component_query(&mut commands, &cutscene_entity_query);
    }
}

pub fn initialise_cutscene_animation_spawn_step(
    mut commands: Commands,
    query: Query<
        (Entity, &CutsceneAnimationSpawn),
        (With<Cinematic>, Added<CutsceneAnimationSpawn>),
    >,
    mut assets_sprite: PxAssets<PxSprite>,
) {
    for (entity, spawn) in query.iter() {
        let sprite = assets_sprite.load_animated(spawn.image_path.clone(), spawn.frame_count);

        let mut entity_commands = commands.entity(entity);
        entity_commands.insert((
            PxSpriteBundle::<Layer> {
                sprite,
                anchor: PxAnchor::BottomLeft,
                layer: Layer::CutsceneBackground,
                ..Default::default()
            },
            PxAnimationBundle {
                duration: PxAnimationDuration::millis_per_animation(
                    spawn.duration.as_millis() as u64
                ),
                on_finish: PxAnimationFinishBehavior::Loop,
                ..Default::default()
            },
            PxSubPosition::from(spawn.start_coordinates),
        ));

        if let Some(tag) = &spawn.tag_o {
            entity_commands.insert(Tag(tag.clone()));
        }
        // TODO Use a function to guarantee removal of both
        entity_commands
            .remove::<CutsceneAnimationSpawn>()
            .remove::<StepStarted>();
    }
}

// pub fn initialise_cutscene_elapsed_step(
//     mut commands: Commands,
//     query: Query<(Entity, &CutsceneElapse), (With<Cinematic>, Added<CutsceneElapse>)>,
//     mut assets_sprite: PxAssets<PxSprite>,
// ) {
//     if let Ok((entity, spawn)) = query.get_single() {}
// }
