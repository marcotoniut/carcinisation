use crate::components::VolumeSettings;
use crate::pixel::{PxAnimationBundle, PxAssets, PxSpriteBundle};
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
    PxAnchor, PxAnimationDirection, PxAnimationDuration, PxAnimationFinishBehavior, PxCanvas,
    PxFrameTransition, PxSprite, PxSubPosition,
};

/// @system Applies the next cutscene act when none is currently active.
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

/// @system Clears timed cutscene segments and optionally their graphics.
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

/// @system Spawns animated cutscene graphics defined for the current act.
pub fn process_cutscene_animations_spawn(
    mut commands: Commands,
    query: Query<
        (Entity, &CutsceneAnimationsSpawn),
        (With<Cinematic>, Added<CutsceneAnimationsSpawn>),
    >,
    assets_sprite: PxAssets<PxSprite>,
    existing_graphics: Query<(Entity, &Layer), With<CutsceneGraphic>>,
) {
    for (entity, spawns) in query.iter() {
        if spawns.spawns.iter().any(|spawn| {
            matches!(
                spawn.layer,
                Layer::CutsceneLayer(CutsceneLayer::Background(_))
            )
        }) {
            let mut to_despawn = Vec::new();
            for (existing, layer) in existing_graphics.iter() {
                if matches!(layer, Layer::CutsceneLayer(CutsceneLayer::Background(_))) {
                    to_despawn.push(existing);
                }
            }
            for id in to_despawn {
                commands.entity(id).despawn();
            }
        }

        for spawn in spawns.spawns.iter() {
            let sprite = assets_sprite.load_animated(spawn.image_path.clone(), spawn.frame_count);

            let mut entity_commands = commands.spawn((
                CutsceneEntity,
                CutsceneGraphic,
                PxSpriteBundle::<Layer> {
                    sprite: sprite.into(),
                    anchor: PxAnchor::BottomLeft,
                    layer: spawn.layer.clone(),
                    canvas: PxCanvas::Camera,
                    ..default()
                },
                PxAnimationBundle::from_parts(
                    PxAnimationDirection::default(),
                    PxAnimationDuration::millis_per_animation(spawn.duration.as_millis() as u64),
                    PxAnimationFinishBehavior::Loop,
                    PxFrameTransition::default(),
                ),
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

/// @system Spawns static cutscene images for the active act.
pub fn process_cutscene_images_spawn(
    mut commands: Commands,
    query: Query<(Entity, &CutsceneImagesSpawn), (With<Cinematic>, Added<CutsceneImagesSpawn>)>,
    assets_sprite: PxAssets<PxSprite>,
    existing_graphics: Query<(Entity, &Layer), With<CutsceneGraphic>>,
) {
    for (entity, spawns) in query.iter() {
        if spawns.spawns.iter().any(|spawn| {
            matches!(
                spawn.layer,
                Layer::CutsceneLayer(CutsceneLayer::Background(_))
            )
        }) {
            let mut to_despawn = Vec::new();
            for (existing, layer) in existing_graphics.iter() {
                if matches!(layer, Layer::CutsceneLayer(CutsceneLayer::Background(_))) {
                    to_despawn.push(existing);
                }
            }
            for id in to_despawn {
                commands.entity(id).despawn();
            }
        }

        for spawn in spawns.spawns.iter() {
            let sprite = assets_sprite.load(spawn.image_path.clone());

            let mut entity_commands = commands.spawn((
                CutsceneEntity,
                CutsceneGraphic,
                PxSpriteBundle::<Layer> {
                    sprite: sprite.into(),
                    anchor: PxAnchor::BottomLeft,
                    layer: spawn.layer.clone(),
                    canvas: PxCanvas::Camera,
                    ..default()
                },
                PxSubPosition::from(spawn.coordinates),
            ));

            if let Some(tag) = &spawn.tag_o {
                entity_commands.insert(Tag(tag.clone()));
            }
        }

        commands.entity(entity).remove::<CutsceneImagesSpawn>();
    }
}

/// @system Starts the configured cutscene music, replacing any previous tracks.
pub fn process_cutscene_music_spawn(
    mut commands: Commands,
    query: Query<(Entity, &CutsceneMusicSpawn), (With<Cinematic>, Added<CutsceneMusicSpawn>)>,
    music_query: Query<Entity, With<Music>>,
    asset_server: Res<AssetServer>,
    volume_settings: Res<VolumeSettings>,
) {
    for (entity, spawn) in query.iter() {
        mark_for_despawn_by_query(&mut commands, &music_query);

        let (player, settings, system_bundle, music_tag) = make_music_bundle(
            &asset_server,
            &volume_settings,
            spawn.music_path.to_string(),
            PlaybackMode::Loop,
        );

        commands.spawn((
            CutsceneEntity,
            player,
            settings,
            system_bundle,
            music_tag,
            Name::new("Cutscene music"),
        ));
        commands.entity(entity).remove::<CutsceneMusicSpawn>();
    }
}

/// @system Stops cutscene music when the act requests it.
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
