use crate::stage::{
    components::placement::{Depth, Speed},
    enemy::components::{
        behavior::{BehaviorBundle, EnemyBehaviorTimer, EnemyBehaviors, EnemyCurrentBehavior},
        CircleAround, Enemy,
    },
    resources::StageTimeDomain,
};
use bevy::prelude::*;
use seldom_pixel::prelude::PxSubPosition;

pub fn check_no_behavior(
    mut commands: Commands,
    mut query: Query<
        (Entity, &mut EnemyBehaviors, &PxSubPosition, &Speed, &Depth),
        (With<Enemy>, Without<EnemyCurrentBehavior>),
    >,
    stage_time: Res<Time<StageTimeDomain>>,
) {
    for (entity, mut behaviors, position, speed, depth) in query.iter_mut() {
        let behavior = behaviors.next_step();

        let duration_o = behavior.get_duration_o();

        let current_behavior = EnemyCurrentBehavior {
            started: stage_time.elapsed(),
            behavior,
        };

        let bundles = current_behavior.get_bundles(stage_time.elapsed(), position, speed.0, *depth);
        match bundles {
            BehaviorBundle::Idle => {}
            BehaviorBundle::LinearTween(linear_movement) => {
                // Insert the LinearTween marker on the enemy
                commands.entity(entity).insert(linear_movement);

                // Spawn tween children to actually drive the movement
                current_behavior.spawn_tween_children(
                    &mut commands,
                    entity,
                    position,
                    speed.0,
                    *depth,
                );
            }
            BehaviorBundle::Attack => {}
            BehaviorBundle::Circle(bundles) => {
                commands.entity(entity).insert(bundles);
            }
            BehaviorBundle::Jump => {
                // Jump currently does not add additional components.
            }
        };

        commands
            .entity(entity)
            .insert(current_behavior)
            .with_children(|p0| {
                if let Some(duration) = duration_o {
                    p0.spawn(EnemyBehaviorTimer::new(entity, duration));
                }
            });
    }
}

pub fn tick_enemy_behavior_timer(
    mut commands: Commands,
    mut query: Query<&mut EnemyBehaviorTimer>,
    stage_time: Res<Time<StageTimeDomain>>,
) {
    for mut behavior in query.iter_mut() {
        behavior.timer.tick(stage_time.delta());
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
