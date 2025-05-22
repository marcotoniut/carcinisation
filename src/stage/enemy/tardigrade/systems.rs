use super::entity::{EnemyTardigrade, EnemyTardigradeAnimation};
use crate::{
    components::DespawnMark,
    game::score::components::Score,
    globals::SCREEN_RESOLUTION_F32_H,
    layer::Layer,
    stage::{
        attack::spawns::boulder_throw::spawn_boulder_throw_attack,
        components::{
            interactive::Dead,
            placement::{Depth, InView},
        },
        enemy::{
            self,
            bundles::{make_enemy_animation_bundle, EnemyAnimationBundle},
            components::{behavior::EnemyCurrentBehavior, *},
            data::tardigrade::TARDIGRADE_ANIMATIONS,
            tardigrade::entity::EnemyTardigradeAttacking,
        },
        resources::StageTime,
    },
    systems::camera::CameraPos,
};
use bevy::prelude::*;
use seldom_pixel::prelude::{PxAnchor, PxSprite, PxSubPosition};
use std::time::Duration;

pub const ENEMY_TARDIGRADE_ATTACK_SPEED: f32 = 3.;

pub fn assign_tardigrade_animation(
    mut commands: Commands,
    query: Query<
        (Entity, &EnemyCurrentBehavior, &PxSubPosition, &Depth),
        (With<EnemyTardigrade>, Without<EnemyTardigradeAnimation>),
    >,
    asset_server: &Res<AssetServer>,
) {
    for (entity, current_behavior, position, depth) in &mut query.iter() {
        let step = current_behavior.behavior.clone();

        let bundle_o = TARDIGRADE_ANIMATIONS.idle.get(depth).map(|animation| {
            (
                EnemyTardigradeAnimation::Idle,
                EnemyAnimationBundle::new(&asset_server, &animation, depth),
            )
        });

        if let Some((animation, enemy_animation_bundle)) = bundle_o {
            commands.entity(entity).insert((
                PxSubPosition(position.0),
                animation,
                enemy_animation_bundle,
            ));
        }
    }
}

pub fn despawn_dead_tardigrade(
    mut commands: Commands,
    mut score: ResMut<Score>,
    asset_server: &Res<AssetServer>,
    query: Query<(Entity, &EnemyTardigrade, &PxSubPosition, &Depth), Added<Dead>>,
) {
    for (entity, tardigrade, position, depth) in query.iter() {
        commands.entity(entity).insert(DespawnMark);

        let animation_o = TARDIGRADE_ANIMATIONS.death.get(depth);

        if let Some(animation) = animation_o {
            let texture = PxSprite(asset_server.load(animation.sprite_path.as_str()));
            // TODO animate animation.frames

            commands.spawn((
                Name::new("Dead - Tardigrade"),
                PxSubPosition::from(position.0),
                texture,
                depth.to_layer(),
                PxAnchor::Center,
                animation.make_animation_bundle(),
            ));
        }

        score.add_u(tardigrade.kill_score());
    }
}

pub fn check_idle_tardigrade(
    mut commands: Commands,
    asset_server: &Res<AssetServer>,
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
                #[cfg(debug_assertions)]
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
