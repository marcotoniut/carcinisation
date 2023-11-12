use crate::stage::{
    components::placement::{Depth, Speed},
    enemy::components::{
        behavior::{BehaviorBundle, EnemyBehaviorTimer, EnemyBehaviors, EnemyCurrentBehavior},
        CircleAround, Enemy,
    },
    resources::StageTime,
};
use bevy::prelude::*;
use seldom_pixel::prelude::PxSubPosition;

pub fn check_no_behavior(
    mut commands: Commands,
    mut query: Query<
        (Entity, &mut EnemyBehaviors, &PxSubPosition, &Speed, &Depth),
        (With<Enemy>, Without<EnemyCurrentBehavior>),
    >,
    stage_time: Res<StageTime>,
) {
    for (entity, mut behaviors, position, speed, depth) in query.iter_mut() {
        let behavior = behaviors.next();

        let duration_o = behavior.get_duration_o();

        let current_behavior = EnemyCurrentBehavior {
            started: stage_time.elapsed,
            behavior,
        };

        let bundles = current_behavior.get_bundles(stage_time.elapsed, position, speed.0, *depth);
        match bundles {
            BehaviorBundle::Idle(bundles) => {
                commands.entity(entity).insert(bundles);
            }
            BehaviorBundle::LinearMovement((
                linear_movement,
                linear_movement_bundle_x,
                linear_movement_bundle_y,
                linear_movement_bundle_z_o,
            )) => {
                let mut entity_commands = commands.entity(entity);
                entity_commands.insert((
                    linear_movement,
                    linear_movement_bundle_x,
                    linear_movement_bundle_y,
                ));
                if let Some(linear_movement_bundle_z) = linear_movement_bundle_z_o {
                    entity_commands.insert(linear_movement_bundle_z);
                }
            }
            BehaviorBundle::Attack(bundles) => {
                commands.entity(entity).insert(bundles);
            }
            BehaviorBundle::Circle(bundles) => {
                commands.entity(entity).insert(bundles);
            }
            BehaviorBundle::Jump(bundles) => {
                commands.entity(entity).insert(bundles);
                // let mut entity_commands = commands.entity(entity).insert((
                //     linear_movement,
                //     linear_movement_bundle_x,
                //     linear_movement_bundle_y,
                // ));
                // if let Some(linear_movement_bundle_z) = linear_movement_bundle_z_o {
                //     entity_commands.insert(linear_movement_bundle_z)
                // } else {
                //     entity_commands
                // }
            }
        };

        commands
            .entity(entity)
            .insert(current_behavior)
            .with_children(|parent| {
                if let Some(duration) = duration_o {
                    parent.spawn(EnemyBehaviorTimer::new(entity, duration));
                }
            });
    }
}

pub fn tick_enemy_behavior_timer(
    mut commands: Commands,
    mut query: Query<&mut EnemyBehaviorTimer>,
    stage_time: Res<StageTime>,
) {
    for mut behavior in query.iter_mut() {
        behavior.timer.tick(stage_time.delta);
        if behavior.timer.just_finished() {
            commands
                .entity(behavior.entity)
                .remove::<EnemyCurrentBehavior>();
        }
    }
}

/**
 * could this be made into a generic?
 */
pub fn tied_components_enemy_current_behavior_circle_around(
    mut commands: Commands,
    query: Query<Entity, (With<CircleAround>, Without<EnemyCurrentBehavior>)>,
) {
    for entity in query.iter() {
        commands.entity(entity).remove::<CircleAround>();
    }
}
