use std::{sync::Arc, time::Duration};

use bevy::prelude::*;
use seldom_pixel::{
    prelude::{PxAnchor, PxAssets, PxSubPosition},
    sprite::{PxSprite, PxSpriteBundle},
};

use crate::{
    components::DespawnMark,
    globals::{CAMERA_CENTER, HALF_SCREEN_RESOLUTION, SCREEN_RESOLUTION},
    plugins::movement::linear::components::{
        LinearDirection, LinearSpeed, LinearTargetPosition, XAxisPosition, YAxisPosition,
    },
    stage::{
        components::{
            Damage, Dead, Depth, DepthProgress, DepthSpeed, Health, Hittable, InView, TargetDepth,
        },
        data::EnemyStep,
        enemy::{bundles::*, components::*, data::mosquito::MOSQUITO_ANIMATIONS},
        events::DepthChanged,
        resources::StageTime,
        score::components::Score,
    },
    systems::camera::CameraPos,
    Layer,
};

pub const ENEMY_MOSQUITO_ATTACK_SPEED: f32 = 3.;

pub fn assign_mosquito_animation(
    mut commands: Commands,
    query: Query<
        (
            Entity,
            &EnemyCurrentBehavior,
            &PxSubPosition,
            &EnemyMosquitoAttacking,
        ),
        (With<EnemyMosquito>, Without<EnemyMosquitoAnimation>),
    >,
    mut assets_sprite: PxAssets<PxSprite>,
) {
    for (entity, behavior, position, attacking) in &mut query.iter() {
        let step = behavior.behavior.clone();

        // HARDCODED depth, should be a component
        let depth = 1;

        let bundle_o = if let Some(attack) = &attacking.attack {
            match attack {
                EnemyMosquitoAttack::Melee => {
                    let animation_o = MOSQUITO_ANIMATIONS.melee_attack.get(&depth);
                    animation_o.map(|animation| {
                        (
                            EnemyMosquitoAnimation::Attack,
                            make_enemy_animation_bundle(&mut assets_sprite, &animation, depth),
                        )
                    })
                }
                EnemyMosquitoAttack::Ranged => {
                    let animation_o = MOSQUITO_ANIMATIONS.fly.get(&depth);
                    animation_o.map(|animation| {
                        (
                            EnemyMosquitoAnimation::Attack,
                            make_enemy_animation_bundle(&mut assets_sprite, &animation, depth),
                        )
                    })
                }
            }
        } else {
            match step {
                EnemyStep::Attack { .. } => {
                    let animation_o = MOSQUITO_ANIMATIONS.fly.get(&depth);
                    animation_o.map(|animation| {
                        (
                            EnemyMosquitoAnimation::Attack,
                            make_enemy_animation_bundle(&mut assets_sprite, &animation, depth),
                        )
                    })
                }
                EnemyStep::Circle { .. } => {
                    let animation_o = MOSQUITO_ANIMATIONS.fly.get(&depth);
                    animation_o.map(|animation| {
                        (
                            EnemyMosquitoAnimation::Fly,
                            make_enemy_animation_bundle(&mut assets_sprite, &animation, depth),
                        )
                    })
                }
                EnemyStep::Idle { .. } => {
                    let animation_o = MOSQUITO_ANIMATIONS.fly.get(&depth);
                    animation_o.map(|animation| {
                        (
                            EnemyMosquitoAnimation::Idle,
                            make_enemy_animation_bundle(&mut assets_sprite, &animation, depth),
                        )
                    })
                }
                EnemyStep::LinearMovement {
                    coordinates,
                    attacking,
                    speed,
                } => {
                    let animation_o = MOSQUITO_ANIMATIONS.fly.get(&depth);
                    animation_o.map(|animation| {
                        (
                            EnemyMosquitoAnimation::Fly,
                            make_enemy_animation_bundle(&mut assets_sprite, &animation, depth),
                        )
                    })
                }
                EnemyStep::Jump {
                    coordinates,
                    attacking,
                    speed,
                } => {
                    let animation_o = MOSQUITO_ANIMATIONS.fly.get(&depth);
                    animation_o.map(|animation| {
                        (
                            EnemyMosquitoAnimation::Fly,
                            make_enemy_animation_bundle(&mut assets_sprite, &animation, depth),
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
    query: Query<(Entity, &EnemyMosquito, &PxSubPosition), Added<Dead>>,
) {
    for (entity, mosquito, position) in query.iter() {
        commands.entity(entity).insert(DespawnMark);

        // HARDCODED depth, should be a component
        let depth = 1;
        let animation_o = MOSQUITO_ANIMATIONS.death.get(&depth);

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
                animation.get_animation_bundle(),
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
    query: Query<
        (
            Entity,
            &EnemyMosquito,
            &mut EnemyMosquitoAttacking,
            &PxSubPosition,
        ),
        With<InView>,
    >,
) {
    let camera_pos = camera_query.get_single().unwrap();
    for (entity, enemy, attacking, position) in &mut query.iter() {
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
                let attack_bundle = make_blood_attack_bundle(&mut assets_sprite, depth.clone());

                let mut attacking = EnemyMosquitoAttacking {
                    attack: Some(EnemyMosquitoAttack::Ranged),
                    last_attack_started: stage_time.elapsed,
                };

                attacking.attack = attacking.attack.clone();
                attacking.last_attack_started = attacking.last_attack_started.clone();

                let target_pos = HALF_SCREEN_RESOLUTION.clone() + camera_pos.0;

                let direction = target_pos - position.0;
                let speed = direction.normalize() * BLOOD_ATTACK_LINE_SPEED;

                let movement_bundle = (
                    XAxisPosition(position.0.x),
                    YAxisPosition(position.0.y),
                    LinearTargetPosition::<StageTime, XAxisPosition>::new(target_pos.x),
                    LinearTargetPosition::<StageTime, YAxisPosition>::new(target_pos.y),
                    LinearDirection::<StageTime, XAxisPosition>::from_delta(
                        target_pos.x - position.0.x,
                    ),
                    LinearDirection::<StageTime, YAxisPosition>::from_delta(
                        target_pos.y - position.0.y,
                    ),
                    LinearSpeed::<StageTime, XAxisPosition>::new(speed.x),
                    LinearSpeed::<StageTime, YAxisPosition>::new(speed.y),
                );

                commands
                    .spawn((
                        Name::new("Attack Blood"),
                        EnemyAttack {},
                        // PursueTargetPosition::<StageTime, PxSubPosition>::new(target_pos),
                        // PursueSpeed::<StageTime, PxSubPosition>::new(
                        //     (target_pos - position.0) * BLOOD_ATTACK_LINE_SPEED,
                        // ),
                        depth,
                        DepthProgress(depth.0.clone() as f32),
                        DepthSpeed(BLOOD_ATTACK_DEPTH_SPEED),
                        TargetDepth(BLOOD_ATTACK_MAX_DEPTH + 1),
                        Damage(BLOOD_ATTACK_DAMAGE),
                        PxSubPosition(position.0),
                        Hittable {},
                        Health(1),
                    ))
                    .insert(attack_bundle)
                    .insert(movement_bundle);
            }
        }
    }
}

pub fn despawn_dead_attacks(
    mut commands: Commands,
    query: Query<(Entity, &EnemyAttack), Added<Dead>>,
) {
    for (entity, _) in query.iter() {
        commands.entity(entity).insert(DespawnMark);
    }
}

/**
 * TODO there's a bug that can happen when DepthChanged is sent on a Dead entity, I suppose
 */
pub fn read_enemy_attack_depth_changed(
    mut commands: Commands,
    mut event_reader: EventReader<DepthChanged>,
    mut assets_sprite: PxAssets<PxSprite>,
) {
    for event in event_reader.iter() {
        if event.depth.0 < BLOOD_ATTACK_MAX_DEPTH {
            let (sprite_bundle, animation_bundle, collision) =
                make_blood_attack_bundle(&mut assets_sprite, event.depth.clone());

            commands
                .entity(event.entity)
                .insert(sprite_bundle)
                .insert(collision)
                .insert(animation_bundle);
        }
    }
}
