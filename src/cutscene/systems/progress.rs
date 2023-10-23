use crate::{
    components::{remove_step, CutsceneElapsedStarted, Music, Tag},
    cutscene::{
        components::{Cinematic, CutsceneEntity, CutsceneGraphic},
        data::*,
        events::CutsceneShutdownEvent,
        resources::{CutsceneProgress, CutsceneTime},
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

pub fn read_step_trigger(
    mut commands: Commands,
    mut cutscene_shutdown_event_writer: EventWriter<CutsceneShutdownEvent>,
    mut progress: ResMut<CutsceneProgress>,
    query: Query<Entity, (With<Cinematic>, Without<CutsceneElapsedStarted>)>,
    data: Res<CutsceneData>,
    time: Res<CutsceneTime>,
) {
    for entity in query.iter() {
        if let Some(act) = data.steps.get(progress.index) {
            progress.index += 1;
            let mut entity_commands = commands.entity(entity);

            entity_commands.insert((
                CutsceneElapse::new(act.elapse),
                CutsceneElapsedStarted(time.elapsed),
            ));

            if let Some(music_despawn) = &act.music_despawn_o {
                entity_commands.insert(music_despawn.clone());
            }
            if let Some(music_spawn) = &act.music_spawn_o {
                entity_commands.insert(music_spawn.clone());
            }
            if let Some(spawn_animations) = &act.spawn_animations_o {
                entity_commands.insert(spawn_animations.clone());
            }
            if act.await_input {
                // for tag in despawn_entities.iter() {
                //     mark_for_despawn_by_component_query(&mut commands, &cutscene_query);
                // }
            }
        } else {
            cutscene_shutdown_event_writer.send(CutsceneShutdownEvent);
        }
    }
}

pub fn check_cutscene_elapsed(
    mut commands: Commands,
    query: Query<(Entity, &CutsceneElapsedStarted, &CutsceneElapse), With<Cinematic>>,
    cutscene_query: Query<Entity, With<CutsceneGraphic>>,
    time: ResMut<CutsceneTime>,
) {
    for (entity, started, elapse) in query.iter() {
        if started.0 + elapse.duration < time.elapsed {
            remove_step::<CutsceneElapse>(&mut commands, entity);
            if elapse.clear_graphics {
                mark_for_despawn_by_component_query(&mut commands, &cutscene_query);
            }
        }
    }
}

pub fn process_cutscene_animations_spawn(
    mut commands: Commands,
    query: Query<
        (Entity, &CutsceneAnimationsSpawn),
        (With<Cinematic>, Added<CutsceneAnimationsSpawn>),
    >,
    mut assets_sprite: PxAssets<PxSprite>,
) {
    for (entity, spawns) in query.iter() {
        for spawn in spawns.spawns.iter() {
            let sprite = assets_sprite.load_animated(spawn.image_path.clone(), spawn.frame_count);

            let mut entity_commands = commands.spawn((
                CutsceneEntity,
                // TODO should I make a bundle that automatically includes both graphic and entity?
                CutsceneGraphic,
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
                PxSubPosition::from(spawn.coordinates),
            ));

            if let Some(tag) = &spawn.tag_o {
                entity_commands.insert(Tag(tag.clone()));
            }
        }

        commands.entity(entity).remove::<CutsceneAnimationsSpawn>();
    }
}

pub fn process_cutscene_music_spawn(
    mut commands: Commands,
    query: Query<(Entity, &CutsceneMusicSpawn), (With<Cinematic>, Added<CutsceneMusicSpawn>)>,
    music_query: Query<Entity, With<Music>>,
    asset_server: Res<AssetServer>,
    volume_settings: Res<VolumeSettings>,
) {
    for (entity, spawn) in query.iter() {
        mark_for_despawn_by_component_query(&mut commands, &music_query);

        let music_bundle = make_music_bundle(
            &asset_server,
            &volume_settings,
            spawn.music_path.to_string(),
            PlaybackMode::Loop,
        );

        commands.spawn((CutsceneEntity, music_bundle, Name::new("Cutscene music")));
        commands.entity(entity).remove::<CutsceneMusicSpawn>();
    }
}

pub fn process_cutscene_music_despawn(
    mut commands: Commands,
    query: Query<(Entity, &CutsceneMusicDespawn), (With<Cinematic>, Added<CutsceneMusicDespawn>)>,
    music_query: Query<Entity, With<Music>>,
) {
    for (entity, despawn) in query.iter() {
        mark_for_despawn_by_component_query(&mut commands, &music_query);
        commands.entity(entity).remove::<CutsceneMusicDespawn>();
    }
}
