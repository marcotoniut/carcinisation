use std::time::Duration;

use bevy::prelude::*;
use seldom_pixel::{
    prelude::{PxAnchor, PxAssets, PxSubPosition},
    sprite::{PxSprite, PxSpriteBundle},
};

use crate::{
    components::DespawnMark,
    globals::HALF_SCREEN_RESOLUTION,
    plugins::movement::linear::components::{
        LinearMovementBundle, TargetingPositionX, TargetingPositionY, TargetingPositionZ,
    },
    stage::{
        attack::{
            components::{
                bundles::make_hovering_attack_animation_bundle, EnemyAttack,
                EnemyHoveringAttackType,
            },
            data::{BLOOD_ATTACK_DAMAGE, BLOOD_ATTACK_DEPTH_SPEED, BLOOD_ATTACK_LINE_SPEED},
        },
        components::{
            damage::InflictsDamage,
            interactive::{Dead, Health, Hittable},
            placement::{Depth, InView},
        },
        data::EnemyStep,
        enemy::{
            bundles::*,
            components::{behavior::EnemyCurrentBehavior, *},
            data::mosquito::MOSQUITO_ANIMATIONS,
        },
        player::components::PLAYER_DEPTH,
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
                EnemyStep::LinearMovement { .. } => {
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
                Name::new("Dead - Mosquito"),
                PxSubPosition::from(position.0),
                PxSpriteBundle::<Layer> {
                    sprite: texture,
                    layer: Layer::Middle(depth),
                    anchor: PxAnchor::Center,
                    ..default()
                },
                animation.make_animation_bundle(),
            ));
        }

        score.add_u(mosquito.kill_score());
    }
}

pub fn check_idle_mosquito(
    mut commands: Commands,
    mut assets_sprite: PxAssets<PxSprite>,
    camera_query: Query<&PxSubPosition, With<CameraPos>>,
    // TODO
    // event_writer: EventWriter<BloodAttackEvent>,
    stage_time: Res<StageTime>,
    query: Query<
        (Entity, &mut EnemyMosquitoAttacking, &PxSubPosition, &Depth),
        (With<InView>, With<EnemyMosquito>),
    >,
) {
    let camera_pos = camera_query.get_single().unwrap();
    for (entity, attacking, position, depth) in &mut query.iter() {
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

                let attack_bundle = make_hovering_attack_animation_bundle(
                    &mut assets_sprite,
                    &EnemyHoveringAttackType::BoulderThrow,
                    depth.clone(),
                );

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
                    LinearMovementBundle::<StageTime, TargetingPositionX>::new(
                        position.0.x,
                        target_pos.x,
                        speed.x,
                    ),
                    LinearMovementBundle::<StageTime, TargetingPositionY>::new(
                        position.0.y,
                        target_pos.y,
                        speed.y,
                    ),
                    LinearMovementBundle::<StageTime, TargetingPositionZ>::new(
                        depth.0.clone() as f32,
                        PLAYER_DEPTH + 1.,
                        BLOOD_ATTACK_DEPTH_SPEED,
                    ),
                );

                commands
                    .spawn((
                        Name::new(format!(
                            "Attack - {}",
                            EnemyHoveringAttackType::BloodShot.get_name()
                        )),
                        EnemyAttack,
                        EnemyHoveringAttackType::BloodShot,
                        // PursueTargetPosition::<StageTime, PxSubPosition>::new(target_pos),
                        // PursueSpeed::<StageTime, PxSubPosition>::new(
                        //     (target_pos - position.0) * BLOOD_ATTACK_LINE_SPEED,
                        // ),
                        depth.clone(),
                        InflictsDamage(BLOOD_ATTACK_DAMAGE),
                        PxSubPosition(position.0),
                        Hittable,
                        Health(1),
                    ))
                    .insert(attack_bundle)
                    .insert(movement_bundle);
            }
        }
    }
}
