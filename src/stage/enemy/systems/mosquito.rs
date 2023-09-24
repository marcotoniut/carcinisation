use std::time::Duration;

use bevy::{audio::PlaybackMode, prelude::*, render::camera};
use seldom_pixel::{
    prelude::{PxAnchor, PxAnimationBundle, PxAnimationDuration, PxAssets, PxSubPosition},
    sprite::{PxSprite, PxSpriteBundle},
};

use crate::{
    globals::SCREEN_RESOLUTION,
    stage::{
        components::{
            Damage, Dead, Depth, DepthProgress, DepthReached, DepthSpeed, Health, LineSpeed,
            TargetDepth, TargetPosition,
        },
        data::EnemyStep,
        enemy::{
            bundles::make_animation_bundle,
            components::{
                EnemyAttack, EnemyMosquito, EnemyMosquitoAnimation, EnemyMosquitoAttack,
                EnemyMosquitoAttacking, BLOOD_ATTACK_DAMAGE, BLOOD_ATTACK_DEPTH_SPEED,
                BLOOD_ATTACK_LINE_SPEED, BLOOD_ATTACK_MAX_DEPTH,
            },
            data::mosquito::MOSQUITO_ANIMATIONS,
            systems::bundles::make_enemy_mosquito_range_attack_bundle,
        },
        events::DepthChanged,
        player::components::Player,
        resources::StageTime,
        score::components::Score,
    },
    systems::{
        audio::{AudioSystemBundle, AudioSystemType},
        camera::CameraPos,
    },
    Layer,
};

pub const ENEMY_MOSQUITO_ATTACK_SPEED: f32 = 3.;

pub fn assign_mosquito_animation(
    mut commands: Commands,
    query: Query<
        (
            Entity,
            &EnemyMosquito,
            &PxSubPosition,
            &EnemyMosquitoAttacking,
        ),
        Without<EnemyMosquitoAnimation>,
    >,
    mut assets_sprite: PxAssets<PxSprite>,
) {
    for (entity, mosquito, position, attacking) in &mut query.iter() {
        let step = mosquito.current_step();

        // HARDCODED depth, should be a component
        let depth = 1;

        let bundle_o = if let Some(attack) = &attacking.attack {
            match attack {
                EnemyMosquitoAttack::Melee => {
                    let animation_o = MOSQUITO_ANIMATIONS.fly.get(depth);
                    animation_o.map(|animation| {
                        (
                            EnemyMosquitoAnimation::Attack,
                            make_animation_bundle(&mut assets_sprite, &animation, depth),
                        )
                    })
                }
                EnemyMosquitoAttack::Ranged => {
                    let animation_o = MOSQUITO_ANIMATIONS.fly.get(depth);
                    animation_o.map(|animation| {
                        (
                            EnemyMosquitoAnimation::Attack,
                            make_animation_bundle(&mut assets_sprite, &animation, depth),
                        )
                    })
                }
            }
        } else {
            match step {
                EnemyStep::Attack { .. } => {
                    let animation_o = MOSQUITO_ANIMATIONS.fly.get(depth);
                    animation_o.map(|animation| {
                        (
                            EnemyMosquitoAnimation::Attack,
                            make_animation_bundle(&mut assets_sprite, &animation, depth),
                        )
                    })
                }
                EnemyStep::Circle { .. } => {
                    let animation_o = MOSQUITO_ANIMATIONS.fly.get(depth);
                    animation_o.map(|animation| {
                        (
                            EnemyMosquitoAnimation::Attack,
                            make_animation_bundle(&mut assets_sprite, &animation, depth),
                        )
                    })
                }
                EnemyStep::Idle { .. } => {
                    let animation_o = MOSQUITO_ANIMATIONS.fly.get(depth);
                    animation_o.map(|animation| {
                        (
                            EnemyMosquitoAnimation::Attack,
                            make_animation_bundle(&mut assets_sprite, &animation, depth),
                        )
                    })
                }
                EnemyStep::Movement {
                    coordinates,
                    attacking,
                    speed,
                } => {
                    let animation_o = MOSQUITO_ANIMATIONS.fly.get(depth);
                    animation_o.map(|animation| {
                        (
                            EnemyMosquitoAnimation::Attack,
                            make_animation_bundle(&mut assets_sprite, &animation, depth),
                        )
                    })
                }
            }
        };

        if let Some((animation, (sprite_bundle, animation_bundle))) = bundle_o {
            commands.entity(entity).insert((
                PxSubPosition(position.0),
                animation,
                sprite_bundle,
                animation_bundle,
            ));
        }
    }
}

pub fn despawn_dead_mosquitoes(
    mut commands: Commands,
    mut assets_sprite: PxAssets<PxSprite>,
    mut score: ResMut<Score>,
    query: Query<(Entity, &EnemyMosquito, &PxSubPosition), With<Dead>>,
) {
    for (entity, mosquito, position) in query.iter() {
        // TODO Can I split this?
        commands.entity(entity).despawn();

        // HARDCODED depth, should be a component
        let depth = 1;
        let animation_o = MOSQUITO_ANIMATIONS.death.get(depth);

        if let Some(animation) = animation_o {
            let texture =
                assets_sprite.load_animated(animation.sprite_path.as_str(), animation.frames);

            commands.spawn((
                Name::new("EnemyMosquito - Dead"),
                PxSubPosition::from(position.0),
                PxSpriteBundle::<Layer> {
                    sprite: texture,
                    layer: Layer::Middle(depth),
                    anchor: PxAnchor::Center,
                    ..default()
                },
                PxAnimationBundle {
                    duration: PxAnimationDuration::millis_per_animation(animation.speed),
                    on_finish: animation.finish_behavior,
                    ..default()
                },
            ));
        }

        score.add_u(mosquito.kill_score());
    }
}

pub fn check_idle_mosquito(
    mut commands: Commands,
    mut assets_sprite: PxAssets<PxSprite>,
    camera_query: Query<&PxSubPosition, With<CameraPos>>,
    stage_time: Res<StageTime>,
    entity_mosquito: Query<
        (
            Entity,
            &EnemyMosquito,
            &mut EnemyMosquitoAttacking,
            &PxSubPosition,
        ),
        // Without<EnemyMosquitoAttacking>,
    >,
) {
    let camera_pos = camera_query.get_single().unwrap();
    for (entity, enemy, mut attacking, position) in &mut entity_mosquito.iter() {
        if attacking.attack.is_none() {
            // if let EnemyStep::Idle { duration } = enemy.current_step() {
            if attacking.last_attack_started
                < stage_time.elapsed + Duration::from_secs_f32(ENEMY_MOSQUITO_ATTACK_SPEED)
            {
                info!("Mosquito {:?} is attacking", entity);
                commands
                    .entity(entity)
                    .remove::<EnemyMosquitoAnimation>()
                    .insert(EnemyMosquitoAttacking {
                        attack: Some(EnemyMosquitoAttack::Ranged),
                        last_attack_started: stage_time.elapsed,
                    });

                let depth = Depth(1);
                let (sprite_bundle, animation_bundle) =
                    make_enemy_mosquito_range_attack_bundle(&mut assets_sprite, depth.clone());

                let mut attacking = EnemyMosquitoAttacking {
                    attack: Some(EnemyMosquitoAttack::Ranged),
                    last_attack_started: stage_time.elapsed,
                };

                attacking.attack = attacking.attack.clone();
                attacking.last_attack_started = attacking.last_attack_started.clone();

                let target_vec = Vec2::new(
                    camera_pos.x + SCREEN_RESOLUTION.x as f32 / 2.,
                    camera_pos.y + SCREEN_RESOLUTION.y as f32 / 2.,
                );

                commands.spawn((
                    Name::new("Attack Blood"),
                    EnemyAttack {},
                    TargetPosition(target_vec),
                    LineSpeed((target_vec - position.0) * BLOOD_ATTACK_LINE_SPEED),
                    depth,
                    DepthProgress(depth.0.clone() as f32),
                    DepthSpeed(BLOOD_ATTACK_DEPTH_SPEED),
                    TargetDepth(BLOOD_ATTACK_MAX_DEPTH + 1),
                    Damage(BLOOD_ATTACK_DAMAGE),
                    attacking,
                    sprite_bundle,
                    animation_bundle,
                    PxSubPosition(position.0),
                ));
            }
        }
    }
}

pub fn read_enemy_attack_depth_changed(
    mut commands: Commands,
    mut event_reader: EventReader<DepthChanged>,
    mut assets_sprite: PxAssets<PxSprite>,
) {
    for event in event_reader.iter() {
        if event.depth.0 < BLOOD_ATTACK_MAX_DEPTH {
            let (sprite_bundle, animation_bundle) =
                make_enemy_mosquito_range_attack_bundle(&mut assets_sprite, event.depth.clone());

            commands
                .entity(event.entity)
                .insert(sprite_bundle)
                .insert(animation_bundle);
        }
    }
}

// TODO simplify
pub fn damage_on_reached(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    depth_query: Query<(Entity, &Damage), With<DepthReached>>,
    mut player_query: Query<&mut Health, With<Player>>,
) {
    for (entity, damage) in &mut depth_query.iter() {
        let sound_effect = asset_server.load("audio/sfx/enemy_melee.ogg");

        commands.spawn((
            AudioBundle {
                source: sound_effect,
                settings: PlaybackSettings {
                    mode: PlaybackMode::Despawn,
                    ..default()
                },
                ..default()
            },
            AudioSystemBundle {
                system_type: AudioSystemType::SFX,
            },
        ));

        commands.entity(entity).despawn();
        for mut health in &mut player_query.iter_mut() {
            let new_health = health.0 as i32 - damage.0 as i32;
            health.0 = new_health.max(0) as u32;
        }
    }
}
