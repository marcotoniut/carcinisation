#![allow(clippy::type_complexity)]

use crate::components::VolumeSettings;
use crate::pixel::{CxAnimationBundle, CxAssets, CxSpriteBundle};
use crate::{
    components::{Cleared, CutsceneElapsedStarted, Music, Tag},
    cutscene::{
        components::{Cinematic, CutsceneEntity, CutsceneGraphic},
        data::{
            CutsceneAnimationsSpawn, CutsceneData, CutsceneElapse, CutsceneImagesSpawn,
            CutsceneLayer, CutsceneMusicDespawn, CutsceneMusicSpawn,
        },
        messages::CutsceneShutdownEvent,
        resources::{CutsceneProgress, CutsceneTimeDomain},
    },
    globals::mark_for_despawn_by_query,
    layer::Layer,
    letterbox::messages::LetterboxMoveEvent,
    systems::spawn::make_music_bundle,
    transitions::trigger_transition,
};
use bevy::{audio::PlaybackMode, prelude::*};
use carapace::{
    prelude::{
        CxAnchor, CxAnimationDirection, CxAnimationDuration, CxAnimationFinishBehavior,
        CxFrameTransition, CxPosition, CxPresentationTransform, CxRenderSpace, CxSprite, WorldPos,
    },
    primitive::{CxPrimitive, CxPrimitiveFill, CxPrimitiveShape},
};

use crate::cutscene::components::{CutsceneAppearAt, RotationFollower, TimelineCurveFollower};
use crate::data::keyframe::{RotationKeyframe, RotationKeyframes};
use crate::globals::SCREEN_RESOLUTION;
use std::time::Duration;

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
    data: Option<Res<CutsceneData>>,
    time: Res<Time<CutsceneTimeDomain>>,
) {
    let Some(data) = data else {
        return;
    };
    let Ok(entity) = query.single() else {
        return;
    };

    if let Some(act) = data.steps.get(progress.index) {
        progress.index += 1;

        if let Some(x) = &act.letterbox_move_o {
            commands.trigger(LetterboxMoveEvent::from(x.clone()));
        }

        let mut entity_commands = commands.entity(entity);

        entity_commands.insert((
            CutsceneElapse::new(act.elapse),
            CutsceneElapsedStarted(time.elapsed()),
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
        if let Some(x) = &act.transition_o {
            trigger_transition(&mut commands, &x.request);
        }
        if let Some(bg) = &act.background_primitive_o {
            commands.spawn((
                CutsceneEntity,
                CutsceneGraphic,
                Name::new("Cutscene Background"),
                CxPrimitive {
                    shape: CxPrimitiveShape::Rect {
                        size: SCREEN_RESOLUTION,
                    },
                    fill: CxPrimitiveFill::Solid(bg.palette_index),
                },
                CxPosition(IVec2::ZERO),
                CxAnchor::BottomLeft,
                bg.layer.clone(),
                CxRenderSpace::Camera,
            ));
        }
        if act.await_input {
            // TODO
        }
    } else {
        let mut entity_commands = commands.entity(entity);
        entity_commands.insert(Cleared);
        commands.trigger(CutsceneShutdownEvent);
    }
}

/// @system Clears timed cutscene segments and optionally their graphics.
pub fn check_cutscene_elapsed(
    mut commands: Commands,
    query: Query<(Entity, &CutsceneElapsedStarted, &CutsceneElapse), With<Cinematic>>,
    cutscene_query: Query<Entity, With<CutsceneGraphic>>,
    time: Res<Time<CutsceneTimeDomain>>,
) {
    for (entity, started, elapse) in query.iter() {
        if started.0 + elapse.duration < time.elapsed() {
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
    assets_sprite: CxAssets<CxSprite>,
    existing_graphics: Query<(Entity, &Layer), With<CutsceneGraphic>>,
) {
    for (entity, spawns) in query.iter() {
        if spawns.spawns.iter().any(|spawn| {
            matches!(
                spawn.layer,
                Layer::CutsceneLayer(CutsceneLayer::Background(_))
            )
        }) {
            for (existing, layer) in existing_graphics.iter() {
                if matches!(layer, Layer::CutsceneLayer(CutsceneLayer::Background(_))) {
                    commands.entity(existing).despawn();
                }
            }
        }

        for spawn in &spawns.spawns {
            let sprite = assets_sprite.load_animated(spawn.image_path.clone(), spawn.frame_count);

            let mut entity_commands = commands.spawn((
                CutsceneEntity,
                CutsceneGraphic,
                CxSpriteBundle::<Layer> {
                    sprite: sprite.into(),
                    anchor: CxAnchor::BottomLeft,
                    layer: spawn.layer.clone(),
                    canvas: CxRenderSpace::Camera,
                    ..default()
                },
                CxAnimationBundle::from_parts(
                    CxAnimationDirection::default(),
                    CxAnimationDuration::millis_per_animation(spawn.duration.as_millis() as u64),
                    CxAnimationFinishBehavior::Loop,
                    CxFrameTransition::default(),
                ),
                WorldPos::from(spawn.coordinates),
            ));

            if let Some(tag) = &spawn.tag_o {
                entity_commands.insert(Tag(tag.clone()));
            }

            if let Some(target_movement) = &spawn.target_movement_o {
                entity_commands.insert(target_movement.make_bundles(spawn.coordinates));
            }

            insert_rotation_keyframes(
                &mut entity_commands,
                &spawn.rotation_keyframes_o,
                spawn.rotation_pivot_o,
                spawn.rotation_offset_deg,
            );
        }

        commands.entity(entity).remove::<CutsceneAnimationsSpawn>();
    }
}

/// @system Spawns static cutscene images for the active act.
pub fn process_cutscene_images_spawn(
    mut commands: Commands,
    query: Query<(Entity, &CutsceneImagesSpawn), (With<Cinematic>, Added<CutsceneImagesSpawn>)>,
    assets_sprite: CxAssets<CxSprite>,
    data: Option<Res<CutsceneData>>,
    existing_graphics: Query<(Entity, &Layer), With<CutsceneGraphic>>,
) {
    let data = data.as_deref();
    for (entity, spawns) in query.iter() {
        if spawns.spawns.iter().any(|spawn| {
            matches!(
                spawn.layer,
                Layer::CutsceneLayer(CutsceneLayer::Background(_))
            )
        }) {
            for (existing, layer) in existing_graphics.iter() {
                if matches!(layer, Layer::CutsceneLayer(CutsceneLayer::Background(_))) {
                    commands.entity(existing).despawn();
                }
            }
        }

        for spawn in &spawns.spawns {
            let sprite = assets_sprite.load(spawn.image_path.clone());

            let uses_timeline = spawn.rotation_time_scale_o.is_some()
                && spawn.rotation_keyframes_o.is_none()
                && data.is_some_and(|d| d.timeline_config_o.is_some());

            let (anchor, world_pos) = if uses_timeline {
                let tc = data.unwrap().timeline_config_o.as_ref().unwrap();
                (
                    CxAnchor::Custom(tc.rotation_pivot),
                    Vec2::new(tc.rotation_position.x as f32, tc.rotation_position.y as f32),
                )
            } else {
                (CxAnchor::BottomLeft, spawn.coordinates)
            };

            let mut entity_commands = commands.spawn((
                CutsceneEntity,
                CutsceneGraphic,
                CxSpriteBundle::<Layer> {
                    sprite: sprite.into(),
                    anchor,
                    layer: spawn.layer.clone(),
                    canvas: CxRenderSpace::Camera,
                    ..default()
                },
                WorldPos::from(world_pos),
            ));

            if let Some(tag) = &spawn.tag_o {
                entity_commands.insert(Tag(tag.clone()));
            }

            if uses_timeline {
                let appear_at = Duration::from_millis(spawn.appear_ms_o.unwrap_or(0));
                entity_commands.insert((
                    TimelineCurveFollower {
                        appear_at,
                        time_scale: spawn.rotation_time_scale_o.unwrap_or(1.0),
                        angle_offset: spawn.rotation_offset_deg.to_radians(),
                    },
                    CxPresentationTransform::default(),
                ));
                if appear_at > Duration::ZERO {
                    entity_commands.insert(Visibility::Hidden);
                }
            } else {
                insert_rotation_keyframes(
                    &mut entity_commands,
                    &spawn.rotation_keyframes_o,
                    spawn.rotation_pivot_o,
                    spawn.rotation_offset_deg,
                );
                if let Some(ref follow_tag) = spawn.follow_rotation_tag_o {
                    entity_commands.insert(RotationFollower {
                        leader_tag: follow_tag.clone(),
                    });
                }
                if let Some(appear_ms) = spawn.appear_ms_o {
                    entity_commands.insert((
                        CutsceneAppearAt(Duration::from_millis(appear_ms)),
                        Visibility::Hidden,
                    ));
                }
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
            spawn.music_path.clone(),
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

/// Inserts rotation keyframes + presentation transform on an entity if configured.
fn insert_rotation_keyframes(
    entity_commands: &mut EntityCommands,
    keyframes_o: &Option<Vec<RotationKeyframe>>,
    pivot_o: Option<Vec2>,
    offset_deg: f32,
) {
    if let Some(keyframes) = keyframes_o {
        entity_commands.insert((
            RotationKeyframes {
                keyframes: keyframes.clone(),
                offset: offset_deg.to_radians(),
            },
            CxPresentationTransform::default(),
        ));
        if let Some(pivot) = pivot_o {
            entity_commands.insert(CxAnchor::Custom(pivot));
        }
    }
}

/// @system Drives absolute rotation keyframes (skips followers — they're handled separately).
pub fn drive_cutscene_rotation_keyframes(
    time: Res<Time<CutsceneTimeDomain>>,
    mut query: Query<
        (&RotationKeyframes, &mut CxPresentationTransform),
        (With<CutsceneGraphic>, Without<RotationFollower>),
    >,
) {
    let elapsed = time.elapsed();
    for (rk, mut pt) in &mut query {
        pt.rotation =
            crate::data::keyframe::evaluate_rotation_keyframes(&rk.keyframes, elapsed) + rk.offset;
    }
}

/// @system Drives followers: rotation = leader's rotation + relative offset keyframes.
pub fn drive_rotation_followers(
    time: Res<Time<CutsceneTimeDomain>>,
    leaders: Query<
        (&Tag, &CxPresentationTransform),
        (With<CutsceneGraphic>, Without<RotationFollower>),
    >,
    mut followers: Query<
        (
            &RotationFollower,
            &RotationKeyframes,
            &mut CxPresentationTransform,
        ),
        With<CutsceneGraphic>,
    >,
) {
    let elapsed = time.elapsed();
    for (follower, rk, mut pt) in &mut followers {
        // Find leader by tag.
        let leader_rotation = leaders
            .iter()
            .find_map(|(tag, leader_pt)| {
                if tag.0.as_str() == follower.leader_tag {
                    Some(leader_pt.rotation)
                } else {
                    None
                }
            })
            .unwrap_or(0.0);

        // Relative offset from keyframes (0 at appear, peaks briefly, returns to 0).
        let relative_offset =
            crate::data::keyframe::evaluate_rotation_keyframes(&rk.keyframes, elapsed) + rk.offset;

        pt.rotation = leader_rotation + relative_offset;
    }
}

/// @system Drives elements following the shared timeline rotation curve.
pub fn drive_timeline_curve_followers(
    time: Res<Time<CutsceneTimeDomain>>,
    data: Option<Res<CutsceneData>>,
    mut query: Query<
        (
            &TimelineCurveFollower,
            &mut CxPresentationTransform,
            &mut Visibility,
        ),
        With<CutsceneGraphic>,
    >,
) {
    let Some(data) = data else {
        return;
    };
    let Some(tc) = &data.timeline_config_o else {
        return;
    };
    let elapsed = time.elapsed();

    for (follower, mut pt, mut visibility) in &mut query {
        if elapsed < follower.appear_at {
            *visibility = Visibility::Hidden;
            continue;
        }
        *visibility = Visibility::Inherited;

        let dt = (elapsed - follower.appear_at).as_secs_f32();
        let scaled_elapsed = follower.appear_at + Duration::from_secs_f32(dt * follower.time_scale);
        pt.rotation = crate::data::keyframe::evaluate_rotation_keyframes(
            &tc.rotation_keyframes,
            scaled_elapsed,
        ) + follower.angle_offset;
    }
}

/// @system Reveals elements when their scheduled appear time is reached.
pub fn check_cutscene_appear_times(
    time: Res<Time<CutsceneTimeDomain>>,
    mut query: Query<(&CutsceneAppearAt, &mut Visibility), With<CutsceneGraphic>>,
) {
    let elapsed = time.elapsed();
    for (appear, mut visibility) in &mut query {
        if elapsed >= appear.0 {
            *visibility = Visibility::Inherited;
        }
    }
}

/// @system Stops cutscene music when the act requests it.
pub fn process_cutscene_music_despawn(
    mut commands: Commands,
    query: Query<(Entity, &CutsceneMusicDespawn), (With<Cinematic>, Added<CutsceneMusicDespawn>)>,
    music_query: Query<Entity, With<Music>>,
) {
    for (entity, _despawn) in query.iter() {
        mark_for_despawn_by_query(&mut commands, &music_query);
        commands.entity(entity).remove::<CutsceneMusicDespawn>();
    }
}
