use crate::stage::{
    components::placement::{Depth, Speed},
    enemy::components::{
        CircleAround, Enemy,
        behavior::{
            BehaviorBundle, EnemyBehaviorTimer, EnemyBehaviors, EnemyCurrentBehavior,
            EnemyStepTweenChild,
        },
    },
    resources::StageTimeDomain,
};
use bevy::{ecs::hierarchy::ChildOf, prelude::*};
use carapace::prelude::PxSubPosition;

/// @system Assigns the next behavior step to enemies with no active behavior.
pub fn check_no_behavior(
    mut commands: Commands,
    mut query: Query<
        (Entity, &mut EnemyBehaviors, &PxSubPosition, &Speed, &Depth),
        (With<Enemy>, Without<EnemyCurrentBehavior>),
    >,
    stage_time: Res<Time<StageTimeDomain>>,
) {
    for (entity, mut behaviors, position, speed, depth) in &mut query {
        let behavior = behaviors.next_step();

        let duration_o = behavior.get_duration_o();

        let current_behavior = EnemyCurrentBehavior {
            started: stage_time.elapsed(),
            behavior,
        };

        let bundles = current_behavior.get_bundles(stage_time.elapsed(), position, speed.0, *depth);
        match bundles {
            BehaviorBundle::Idle | BehaviorBundle::Attack => {}
            BehaviorBundle::Jump(jump_movement) => {
                // Spawn tween children to drive the jump arc movement.
                current_behavior.spawn_tween_children(
                    &mut commands,
                    entity,
                    position,
                    speed.0,
                    *depth,
                );
                commands.entity(entity).insert(jump_movement);
            }
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
            BehaviorBundle::Circle(bundles) => {
                commands.entity(entity).insert(bundles);
            }
        }

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

/// @system Ticks behavior timers and clears the current behavior when time expires.
pub fn tick_enemy_behavior_timer(
    mut commands: Commands,
    mut query: Query<&mut EnemyBehaviorTimer>,
    stage_time: Res<Time<StageTimeDomain>>,
) {
    for mut behavior in &mut query {
        behavior.timer.tick(stage_time.delta());
        if behavior.timer.just_finished() {
            commands
                .entity(behavior.entity)
                .remove::<EnemyCurrentBehavior>();
        }
    }
}

/// @system Removes `CircleAround` when the owning behavior ends.
// TODO could this be made into a generic?
pub fn tied_components_enemy_current_behavior_circle_around(
    mut commands: Commands,
    query: Query<Entity, (With<CircleAround>, Without<EnemyCurrentBehavior>)>,
) {
    for entity in query.iter() {
        commands.entity(entity).remove::<CircleAround>();
    }
}

/// @system Despawns tween children when their parent enemy's behavior ends.
///
/// Tween children (`EnemyStepTweenChild`) are spawned to drive `LinearTween` movement
/// but are not part of Bevy's hierarchy system. Without cleanup, they orphan when
/// behaviors transition, causing memory leaks.
///
/// This system:
/// 1. Queries all tween children with their parent reference (`ChildOf`)
/// 2. Checks if parent still has `EnemyCurrentBehavior`
/// 3. Despawns orphaned children whose parent behavior ended
pub fn cleanup_orphaned_tween_children(
    mut commands: Commands,
    tween_children_query: Query<(Entity, &ChildOf), With<EnemyStepTweenChild>>,
    parent_query: Query<(), With<EnemyCurrentBehavior>>,
) {
    for (child_entity, child_of) in &tween_children_query {
        // Check if parent still has active behavior
        if parent_query.get(child_of.0).is_err() {
            // Parent behavior ended, despawn orphaned tween child
            commands.entity(child_entity).despawn();
        }
    }
}
