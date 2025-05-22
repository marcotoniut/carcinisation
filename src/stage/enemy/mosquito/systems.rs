use super::entity::*;
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
            components::{behavior::EnemyCurrentBehavior, *},
            data::{
                mosquito::MOSQUITO_ANIMATIONS,
                steps::{EnemyStep, JumpEnemyStep},
            },
        },
        resources::StageTime,
    },
    systems::camera::CameraPos,
};
use bevy::prelude::*;
use seldom_pixel::prelude::{PxAnchor, PxSprite, PxSubPosition};
use std::time::Duration;

pub const ENEMY_MOSQUITO_ATTACK_SPEED: f32 = 3.;

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
    asset_server: &Res<AssetServer>,
) {
    for (entity, behavior, position, attacking, depth) in &mut query.iter() {
        let step = behavior.behavior.clone();

        let bundle_o = if let Some(attack) = &attacking.attack {
            match attack {
                EnemyMosquitoAttack::Melee => {
                    let animation_o = MOSQUITO_ANIMATIONS.melee_attack.get(depth);
                    animation_o.map(|animation| {
                        (
                            EnemyMosquitoAnimation::Attack,
                            EnemyAnimationBundle::new(&asset_server, &animation, depth),
                        )
                    })
                }
                EnemyMosquitoAttack::Ranged => {
                    let animation_o = MOSQUITO_ANIMATIONS.fly.get(depth);
                    animation_o.map(|animation| {
                        (
                            EnemyMosquitoAnimation::Attack,
                            EnemyAnimationBundle::new(&asset_server, &animation, depth),
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
                            EnemyAnimationBundle::new(&asset_server, &animation, depth),
                        )
                    })
                }
                EnemyStep::Circle { .. } => {
                    let animation_o = MOSQUITO_ANIMATIONS.fly.get(depth);
                    animation_o.map(|animation| {
                        (
                            EnemyMosquitoAnimation::Fly,
                            EnemyAnimationBundle::new(&asset_server, &animation, depth),
                        )
                    })
                }
                EnemyStep::Idle { .. } => {
                    let animation_o = MOSQUITO_ANIMATIONS.fly.get(depth);
                    animation_o.map(|animation| {
                        (
                            EnemyMosquitoAnimation::Idle,
                            EnemyAnimationBundle::new(&asset_server, &animation, depth),
                        )
                    })
                }
                EnemyStep::LinearMovement { .. } => {
                    let animation_o = MOSQUITO_ANIMATIONS.fly.get(depth);
                    animation_o.map(|animation| {
                        (
                            EnemyMosquitoAnimation::Fly,
                            EnemyAnimationBundle::new(&asset_server, &animation, depth),
                        )
                    })
                }
                EnemyStep::Jump(JumpEnemyStep {
                    coordinates,
                    attacking,
                    speed,
                    ..
                }) => {
                    let animation_o = MOSQUITO_ANIMATIONS.fly.get(depth);
                    animation_o.map(|animation| {
                        (
                            EnemyMosquitoAnimation::Fly,
                            EnemyAnimationBundle::new(&asset_server, &animation, depth),
                        )
                    })
                }
            }
        };

        if let Some((animation, enemy_animation_bundle)) = bundle_o {
            commands.entity(entity).insert((
                PxSubPosition(position.0),
                animation,
                enemy_animation_bundle,
            ));
        }
    }
}

pub fn despawn_dead_mosquitoes(
    mut commands: Commands,
    mut score: ResMut<Score>,
    query: Query<(Entity, &EnemyMosquito, &PxSubPosition, &Depth), Added<Dead>>,
) {
    for (entity, mosquito, position, depth) in query.iter() {
        commands.entity(entity).insert(DespawnMark);

        let animation_o = MOSQUITO_ANIMATIONS.death.get(depth);

        if let Some(animation) = animation_o {
            let texture = PxSprite(assets_sprite.load(animation.sprite_path.as_str()));
            // TODO animate animation.frames

            commands.spawn((
                Name::new("Dead - Mosquito"),
                PxSubPosition::from(position.0),
                texture,
                depth.to_layer(),
                PxAnchor::Center,
                animation.make_animation_bundle(),
            ));
        }

        score.add_u(mosquito.kill_score());
    }
}

pub fn check_idle_mosquito(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
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
                #[cfg(debug_assertions)]
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
                    &asset_server,
                    &stage_time,
                    SCREEN_RESOLUTION_F32_H.clone() + camera_pos.0,
                    position.0,
                    depth,
                );
            }
        }
    }
}
