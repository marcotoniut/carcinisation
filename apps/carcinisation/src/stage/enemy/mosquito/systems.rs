use super::entity::*;
use crate::pixel::{PxAssets, PxSpriteBundle};
use crate::{
    components::DespawnMark,
    game::score::components::Score,
    globals::SCREEN_RESOLUTION_F32_H,
    layer::Layer,
    stage::{
        attack::spawns::blood_shot::spawn_blood_shot_attack,
        components::{
            interactive::Dead,
            placement::{Depth, InView},
        },
        enemy::{
            bundles::*,
            components::behavior::EnemyCurrentBehavior,
            data::{
                mosquito::MOSQUITO_ANIMATIONS,
                steps::{EnemyStep, JumpEnemyStep},
            },
        },
        resources::StageTimeDomain,
    },
    systems::camera::CameraPos,
};
use bevy::prelude::*;
use seldom_pixel::prelude::{PxAnchor, PxSprite, PxSubPosition};
use std::time::Duration;

pub const ENEMY_MOSQUITO_ATTACK_SPEED: f32 = 3.;

/// @system Picks the correct mosquito sprite for the current behavior and depth.
pub fn assign_mosquito_animation(
    mut commands: Commands,
    query: Query<
        (
            Entity,
            &EnemyCurrentBehavior,
            &PxSubPosition,
            &EnemyMosquitoAttacking,
            &Depth,
        ),
        (With<EnemyMosquito>, Without<EnemyMosquitoAnimation>),
    >,
    mut assets_sprite: PxAssets<PxSprite>,
) {
    for (entity, behavior, position, attacking, depth) in &mut query.iter() {
        let step = behavior.behavior;

        let bundle_o = if let Some(attack) = &attacking.attack {
            match attack {
                EnemyMosquitoAttack::Melee => {
                    let animation_o = MOSQUITO_ANIMATIONS.melee_attack.get(depth);
                    animation_o.map(|animation| {
                        (
                            EnemyMosquitoAnimation::Attack,
                            make_enemy_animation_bundle(&mut assets_sprite, animation, depth),
                        )
                    })
                }
                EnemyMosquitoAttack::Ranged => {
                    let animation_o = MOSQUITO_ANIMATIONS.fly.get(depth);
                    animation_o.map(|animation| {
                        (
                            EnemyMosquitoAnimation::Attack,
                            make_enemy_animation_bundle(&mut assets_sprite, animation, depth),
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
                            make_enemy_animation_bundle(&mut assets_sprite, animation, depth),
                        )
                    })
                }
                EnemyStep::Circle { .. } => {
                    let animation_o = MOSQUITO_ANIMATIONS.fly.get(depth);
                    animation_o.map(|animation| {
                        (
                            EnemyMosquitoAnimation::Fly,
                            make_enemy_animation_bundle(&mut assets_sprite, animation, depth),
                        )
                    })
                }
                EnemyStep::Idle { .. } => {
                    let animation_o = MOSQUITO_ANIMATIONS.fly.get(depth);
                    animation_o.map(|animation| {
                        (
                            EnemyMosquitoAnimation::Idle,
                            make_enemy_animation_bundle(&mut assets_sprite, animation, depth),
                        )
                    })
                }
                EnemyStep::LinearTween { .. } => {
                    let animation_o = MOSQUITO_ANIMATIONS.fly.get(depth);
                    animation_o.map(|animation| {
                        (
                            EnemyMosquitoAnimation::Fly,
                            make_enemy_animation_bundle(&mut assets_sprite, animation, depth),
                        )
                    })
                }
                EnemyStep::Jump(JumpEnemyStep { .. }) => {
                    let animation_o = MOSQUITO_ANIMATIONS.fly.get(depth);
                    animation_o.map(|animation| {
                        (
                            EnemyMosquitoAnimation::Fly,
                            make_enemy_animation_bundle(&mut assets_sprite, animation, depth),
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

/// @system Spawns a death animation and awards score when a mosquito dies.
pub fn despawn_dead_mosquitoes(
    mut commands: Commands,
    assets_sprite: PxAssets<PxSprite>,
    mut score: ResMut<Score>,
    query: Query<(Entity, &EnemyMosquito, &PxSubPosition, &Depth), Added<Dead>>,
) {
    for (entity, mosquito, position, depth) in query.iter() {
        commands.entity(entity).insert(DespawnMark);

        let animation_o = MOSQUITO_ANIMATIONS.death.get(depth);

        if let Some(animation) = animation_o {
            let texture =
                assets_sprite.load_animated(animation.sprite_path.as_str(), animation.frames);

            commands.spawn((
                Name::new("Dead - Mosquito"),
                PxSubPosition::from(position.0),
                PxSpriteBundle::<Layer> {
                    sprite: texture.into(),
                    layer: depth.to_layer(),
                    anchor: PxAnchor::Center,
                    ..default()
                },
                animation.make_animation_bundle(),
            ));
        }

        score.add_u(mosquito.kill_score());
    }
}

/// @system Fires ranged attacks from idle in-view mosquitoes on a cooldown.
pub fn check_idle_mosquito(
    mut commands: Commands,
    mut assets_sprite: PxAssets<PxSprite>,
    camera_query: Query<&PxSubPosition, With<CameraPos>>,
    // TODO
    // event_writer: MessageWriter<BloodAttackEvent>,
    stage_time: Res<Time<StageTimeDomain>>,
    query: Query<
        (Entity, &mut EnemyMosquitoAttacking, &PxSubPosition, &Depth),
        (With<InView>, With<EnemyMosquito>),
    >,
) {
    let camera_pos = camera_query.single().unwrap();
    for (entity, attacking, position, depth) in &mut query.iter() {
        if attacking.attack.is_none() {
            // if let EnemyStep::Idle { duration } = enemy.current_step() {
            if attacking.last_attack_started
                < stage_time.elapsed() + Duration::from_secs_f32(ENEMY_MOSQUITO_ATTACK_SPEED)
            {
                #[cfg(debug_assertions)]
                info!("Mosquito {:?} is attacking", entity);

                commands
                    .entity(entity)
                    .remove::<EnemyMosquitoAnimation>()
                    .insert(EnemyMosquitoAttacking {
                        attack: Some(EnemyMosquitoAttack::Ranged),
                        last_attack_started: stage_time.elapsed(),
                    });

                spawn_blood_shot_attack(
                    &mut commands,
                    &mut assets_sprite,
                    &stage_time,
                    *SCREEN_RESOLUTION_F32_H + camera_pos.0,
                    position.0,
                    depth,
                );
            }
        }
    }
}
