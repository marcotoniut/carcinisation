use std::time::Duration;

use bevy::prelude::*;
use seldom_pixel::{
    prelude::{PxAnchor, PxAssets, PxSubPosition},
    sprite::{PxSprite, PxSpriteBundle},
};

use crate::{
    components::DespawnMark,
    game::score::components::Score,
    globals::HALF_SCREEN_RESOLUTION,
    stage::{
        attack::spawns::blood_shot::spawn_blood_shot_attack,
        components::{
            interactive::Dead,
            placement::{Depth, InView},
        },
        data::EnemyStep,
        enemy::{
            bundles::*,
            components::{behavior::EnemyCurrentBehavior, *},
            data::mosquito::MOSQUITO_ANIMATIONS,
        },
        resources::StageTime,
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

                spawn_blood_shot_attack(
                    &mut commands,
                    &mut assets_sprite,
                    &stage_time,
                    HALF_SCREEN_RESOLUTION.clone() + camera_pos.0,
                    position.0,
                    depth,
                );
            }
        }
    }
}
