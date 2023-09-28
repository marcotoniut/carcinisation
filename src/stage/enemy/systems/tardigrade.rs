use std::{ops::Add, time::Duration};

use bevy::prelude::*;
use seldom_pixel::{
    prelude::{PxAnchor, PxAssets, PxSubPosition},
    sprite::{PxSprite, PxSpriteBundle},
};

use crate::{
    components::DespawnMark,
    globals::{CAMERA_CENTER, SCREEN_RESOLUTION},
    plugins::movement::pursue::components::{PursueSpeed, PursueTargetPosition},
    stage::{
        components::{
            Damage, Dead, Depth, DepthProgress, DepthSpeed, Health, Hittable, InView, TargetDepth,
        },
        enemy::{
            bundles::{make_blood_attack_bundle, make_enemy_animation_bundle},
            components::*,
            data::tardigrade::TARDIGRADE_ANIMATIONS,
        },
        resources::StageTime,
        score::components::Score,
    },
    systems::camera::CameraPos,
    Layer,
};

pub const ENEMY_TARDIGRADE_ATTACK_SPEED: f32 = 3.;

pub fn assign_tardigrade_animation(
    mut commands: Commands,
    query: Query<
        (Entity, &EnemyCurrentBehavior, &PxSubPosition),
        (With<EnemyTardigrade>, Without<EnemyTardigradeAnimation>),
    >,
    mut assets_sprite: PxAssets<PxSprite>,
) {
    for (entity, current_behavior, position) in &mut query.iter() {
        let step = current_behavior.behavior.clone();

        // HARDCODED depth, should be a component
        let depth = 1;

        let bundle_o = TARDIGRADE_ANIMATIONS.idle.get(&depth).map(|animation| {
            (
                EnemyTardigradeAnimation::Idle,
                make_enemy_animation_bundle(&mut assets_sprite, &animation, depth),
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
    query: Query<(Entity, &EnemyTardigrade, &PxSubPosition), Added<Dead>>,
) {
    for (entity, mosquito, position) in query.iter() {
        commands.entity(entity).insert(DespawnMark);

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
            &EnemyTardigrade,
            &mut EnemyTardigradeAttacking,
            &PxSubPosition,
        ),
        With<InView>,
    >,
) {
    let camera_pos = camera_query.get_single().unwrap();
    for (entity, enemy, attacking, position) in &mut query.iter() {
        if attacking.attack == true {
            // if let EnemyStep::Idle { duration } = enemy.current_step() {
            if attacking.last_attack_started
                < stage_time.elapsed + Duration::from_secs_f32(ENEMY_TARDIGRADE_ATTACK_SPEED)
            {
                info!("Tardigrade {:?} is attacking", entity);
                commands
                    .entity(entity)
                    .remove::<EnemyTardigradeAnimation>()
                    .insert(EnemyTardigradeAttacking {
                        attack: true,
                        last_attack_started: stage_time.elapsed,
                    });

                let depth = Depth(1);
                let attack_bundle = make_blood_attack_bundle(&mut assets_sprite, depth.clone());

                let mut attacking = EnemyTardigradeAttacking {
                    attack: true,
                    last_attack_started: stage_time.elapsed,
                };

                attacking.attack = attacking.attack.clone();
                attacking.last_attack_started = attacking.last_attack_started.clone();

                let target_pos = CAMERA_CENTER.clone() + camera_pos.0;

                commands
                    .spawn((
                        Name::new("Attack Blood"),
                        EnemyAttack {},
                        // TODO bundle
                        PursueTargetPosition::<StageTime, PxSubPosition>::new(target_pos),
                        PursueSpeed::<StageTime, PxSubPosition>::new(
                            (target_pos - position.0) * BLOOD_ATTACK_LINE_SPEED,
                        ),
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
