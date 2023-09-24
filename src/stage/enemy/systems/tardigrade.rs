use std::time::Duration;

use bevy::prelude::*;
use seldom_pixel::{
    prelude::{PxAnchor, PxAssets, PxSubPosition},
    sprite::{PxSprite, PxSpriteBundle},
};

use crate::{
    globals::SCREEN_RESOLUTION,
    stage::{
        components::{
            Damage, Dead, Depth, DepthProgress, DepthSpeed, Health, Hittable, InView, LineSpeed,
            TargetDepth, TargetPosition,
        },
        enemy::{
            bundles::{make_animation_bundle, make_blood_attack_bundle},
            components::{
                EnemyAttack, EnemyMosquito, EnemyMosquitoAnimation, EnemyMosquitoAttack,
                EnemyMosquitoAttacking, EnemyTardigrade, EnemyTardigradeAnimation,
                BLOOD_ATTACK_DAMAGE, BLOOD_ATTACK_DEPTH_SPEED, BLOOD_ATTACK_LINE_SPEED,
                BLOOD_ATTACK_MAX_DEPTH,
            },
            data::tardigrade::TARDIGRADE_ANIMATIONS,
        },
        resources::StageTime,
        score::components::Score,
    },
    systems::camera::CameraPos,
    Layer,
};

pub const ENEMY_MOSQUITO_ATTACK_SPEED: f32 = 3.;

pub fn assign_tardigrade_animation(
    mut commands: Commands,
    query: Query<(Entity, &EnemyTardigrade, &PxSubPosition), Without<EnemyTardigradeAnimation>>,
    mut assets_sprite: PxAssets<PxSprite>,
) {
    for (entity, mosquito, position) in &mut query.iter() {
        let step = mosquito.current_step();

        // HARDCODED depth, should be a component
        let depth = 1;

        let bundle_o = TARDIGRADE_ANIMATIONS.idle.get(&depth).map(|animation| {
            (
                EnemyTardigradeAnimation::Idle,
                make_animation_bundle(&mut assets_sprite, &animation, depth),
            )
        });

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

pub fn despawn_dead_tardigrade(
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
        let animation_o = TARDIGRADE_ANIMATIONS.death.get(&depth);

        if let Some(animation) = animation_o {
            let texture =
                assets_sprite.load_animated(animation.sprite_path.as_str(), animation.frames);

            commands.spawn((
                Name::new("EnemyTardigrade - Dead"),
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

pub fn check_idle_tardigrade(
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
    for (entity, enemy, mut attacking, position) in &mut query.iter() {
        if attacking.attack.is_none() {
            // if let EnemyStep::Idle { duration } = enemy.current_step() {
            if attacking.last_attack_started
                < stage_time.elapsed + Duration::from_secs_f32(ENEMY_MOSQUITO_ATTACK_SPEED)
            {
                info!("Tardigrade {:?} is attacking", entity);
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

                let target_vec = Vec2::new(
                    camera_pos.x + SCREEN_RESOLUTION.x as f32 / 2.,
                    camera_pos.y + SCREEN_RESOLUTION.y as f32 / 2.,
                );

                commands
                    .spawn((
                        Name::new("Attack Blood"),
                        EnemyAttack {},
                        TargetPosition(target_vec),
                        LineSpeed((target_vec - position.0) * BLOOD_ATTACK_LINE_SPEED),
                        depth,
                        DepthProgress(depth.0.clone() as f32),
                        DepthSpeed(BLOOD_ATTACK_DEPTH_SPEED),
                        TargetDepth(BLOOD_ATTACK_MAX_DEPTH + 1),
                        Damage(BLOOD_ATTACK_DAMAGE),
                        PxSubPosition(position.0),
                        Hittable {},
                        Health(1),
                    ))
                    .insert(attack_bundle);
            }
        }
    }
}
