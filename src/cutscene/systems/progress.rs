use crate::components::VolumeSettings;
use crate::{
    components::{Cleared, CutsceneElapsedStarted, Music, Tag},
    cutscene::{
        components::{Cinematic, CutsceneEntity, CutsceneGraphic},
        data::*,
        events::CutsceneShutdownTrigger,
        resources::{CutsceneProgress, CutsceneTime},
    },
    globals::mark_for_despawn_by_query,
    layer::Layer,
    letterbox::events::LetterboxMoveTrigger,
    systems::spawn::make_music_bundle,
};
use bevy::{audio::PlaybackMode, prelude::*};
use seldom_pixel::prelude::{
    PxAnchor, PxAnimation, PxAnimationDuration, PxAnimationFinishBehavior, PxSprite, PxSubPosition,
};

pub fn read_step_trigger(
    mut commands: Commands,
    mut progress: ResMut<CutsceneProgress>,
    query: Query<
        Entity,
        (
            With<Cinematic>,
            Without<CutsceneElapsedStarted>,
            Without<Cleared>,
        ),
    >,
    data: Res<CutsceneData>,
    time: Res<CutsceneTime>,
) {
    for entity in query.iter() {
        if let Some(act) = data.steps.get(progress.index) {
            progress.index += 1;

            if let Some(x) = &act.letterbox_move_o {
                commands.trigger(LetterboxMoveTrigger::from(x.clone()));
            }

            let mut entity_commands = commands.entity(entity);

            entity_commands.insert((
                CutsceneElapse::new(act.elapse),
                CutsceneElapsedStarted(time.elapsed),
            ));

            if let Some(x) = &act.music_despawn_o {
                entity_commands.insert(x.clone());
            }
            if let Some(x) = &act.music_spawn_o {
                entity_commands.insert(x.clone());
            }
            if let Some(x) = &act.spawn_animations_o {
                entity_commands.insert(x.clone());
            }
            if let Some(x) = &act.spawn_images_o {
                entity_commands.insert(x.clone());
            }
            if act.await_input {
                // TODO
            }
        } else {
            let mut entity_commands = commands.entity(entity);
            entity_commands.insert(Cleared);
            commands.trigger(CutsceneShutdownTrigger);
        }
    }
}

pub fn check_cutscene_elapsed(
    mut commands: Commands,
    query: Query<(Entity, &CutsceneElapsedStarted, &CutsceneElapse), With<Cinematic>>,
    cutscene_query: Query<Entity, With<CutsceneGraphic>>,
    time: Res<CutsceneTime>,
) {
    for (entity, started, elapse) in query.iter() {
        if started.0 + elapse.duration < time.elapsed {
            commands
                .entity(entity)
                .remove::<CutsceneElapse>()
                .remove::<CutsceneElapsedStarted>();

            if elapse.clear_graphics {
                mark_for_despawn_by_query(&mut commands, &cutscene_query);
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
    asset_server: Res<AssetServer>,
) {
    for (entity, spawns) in query.iter() {
        for spawn in spawns.spawns.iter() {
            let sprite = PxSprite(asset_server.load(spawn.image_path.clone()));
            // TODO animate spawn.frame_count

            let mut entity_commands = commands.spawn((
                CutsceneEntity,
                CutsceneGraphic,
                sprite,
                PxAnchor::BottomLeft,
                spawn.layer.clone(),
                PxAnimation {
                    duration: PxAnimationDuration::millis_per_animation(
                        spawn.duration.as_millis() as u64
                    ),
                    on_finish: PxAnimationFinishBehavior::Loop,
                    ..default()
                },
                PxSubPosition::from(spawn.coordinates),
            ));

            if let Some(tag) = &spawn.tag_o {
                entity_commands.insert(Tag(tag.clone()));
            }

            if let Some(target_movement) = &spawn.target_movement_o {
                entity_commands.insert(target_movement.make_bundles(spawn.coordinates.clone()));
            }
        }

        commands.entity(entity).remove::<CutsceneAnimationsSpawn>();
    }
}

pub fn process_cutscene_images_spawn(
    mut commands: Commands,
    query: Query<(Entity, &CutsceneImagesSpawn), (With<Cinematic>, Added<CutsceneImagesSpawn>)>,
    asset_server: Res<AssetServer>,
) {
    for (entity, spawns) in query.iter() {
        for spawn in spawns.spawns.iter() {
            let sprite = asset_server.load(spawn.image_path.clone());

            let mut entity_commands = commands.spawn((
                CutsceneEntity,
                CutsceneGraphic,
                sprite,
                PxAnchor::BottomLeft,
                spawn.layer.clone(),
                PxSubPosition::from(spawn.coordinates),
            ));

            if let Some(tag) = &spawn.tag_o {
                entity_commands.insert(Tag(tag.clone()));
            }
        }

        commands.entity(entity).remove::<CutsceneImagesSpawn>();
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
        mark_for_despawn_by_query(&mut commands, &music_query);

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
        mark_for_despawn_by_query(&mut commands, &music_query);
        commands.entity(entity).remove::<CutsceneMusicDespawn>();
    }
}
