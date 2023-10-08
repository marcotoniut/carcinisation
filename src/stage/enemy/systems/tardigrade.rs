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
        attack::spawns::boulder_throw::spawn_boulder_throw_attack,
        components::{
            interactive::Dead,
            placement::{Depth, InView},
        },
        enemy::{
            bundles::make_enemy_animation_bundle,
            components::{behavior::EnemyCurrentBehavior, *},
            data::tardigrade::TARDIGRADE_ANIMATIONS,
        },
        resources::StageTime,
    },
    systems::camera::CameraPos,
    Layer,
};

pub const ENEMY_TARDIGRADE_ATTACK_SPEED: f32 = 3.;

pub fn assign_tardigrade_animation(
    mut commands: Commands,
    query: Query<
        (Entity, &EnemyCurrentBehavior, &PxSubPosition, &Depth),
        (With<EnemyTardigrade>, Without<EnemyTardigradeAnimation>),
    >,
    mut assets_sprite: PxAssets<PxSprite>,
) {
    for (entity, current_behavior, position, depth) in &mut query.iter() {
        let step = current_behavior.behavior.clone();

        let bundle_o = TARDIGRADE_ANIMATIONS.idle.get(&depth.0).map(|animation| {
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
    query: Query<(Entity, &EnemyTardigrade, &PxSubPosition, &Depth), Added<Dead>>,
) {
    for (entity, tardigrade, position, depth) in query.iter() {
        commands.entity(entity).insert(DespawnMark);

        let animation_o = TARDIGRADE_ANIMATIONS.death.get(&depth.0);

        if let Some(animation) = animation_o {
            let texture =
                assets_sprite.load_animated(animation.sprite_path.as_str(), animation.frames);

            commands.spawn((
                Name::new("Dead - Tardigrade"),
                PxSubPosition::from(position.0),
                PxSpriteBundle::<Layer> {
                    sprite: texture,
                    layer: Layer::Middle(depth.0),
                    anchor: PxAnchor::Center,
                    ..default()
                },
                animation.make_animation_bundle(),
            ));
        }

        score.add_u(tardigrade.kill_score());
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
            &Depth,
        ),
        With<InView>,
    >,
) {
    let camera_pos = camera_query.get_single().unwrap();
    for (entity, enemy, attacking, position, depth) in &mut query.iter() {
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

                spawn_boulder_throw_attack(
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
