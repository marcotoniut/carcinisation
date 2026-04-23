use crate::stage::{
    components::placement::Speed,
    enemy::components::{
        CircleAround, Enemy, EnemyContinuousDepth,
        behavior::{
            BehaviorBundle, EnemyBehaviorTimer, EnemyBehaviors, EnemyCurrentBehavior,
            EnemyStepTweenChild,
        },
    },
    resources::StageTimeDomain,
};
use bevy::{ecs::hierarchy::ChildOf, prelude::*};
use carapace::prelude::WorldPos;

/// Seeds continuous enemy depth from the current gameplay bucket when missing.
///
/// Central spawn paths should already insert [`EnemyContinuousDepth`], but
/// this keeps direct debug/test spawns from silently falling back to the old
/// transient-only model.
pub fn ensure_enemy_continuous_depth(world: &mut World) {
    let mut query = world.query_filtered::<
        (Entity, &crate::stage::components::placement::Depth),
        (With<Enemy>, Without<EnemyContinuousDepth>),
    >();
    let missing_depths = query
        .iter(world)
        .map(|(entity, depth)| (entity, *depth))
        .collect::<Vec<_>>();

    for (entity, depth) in missing_depths {
        world
            .entity_mut(entity)
            .insert(EnemyContinuousDepth::from_depth(depth));
    }
}

/// @system Assigns the next behavior step to enemies with no active behavior.
pub fn check_no_behavior(
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &mut EnemyBehaviors,
            &WorldPos,
            &Speed,
            &EnemyContinuousDepth,
        ),
        (With<Enemy>, Without<EnemyCurrentBehavior>),
    >,
    stage_time: Res<Time<StageTimeDomain>>,
) {
    for (entity, mut behaviors, position, speed, continuous_depth) in &mut query {
        let behavior = behaviors.next_step();

        let duration_o = behavior.get_duration_o();

        let current_behavior = EnemyCurrentBehavior {
            started: stage_time.elapsed(),
            behavior,
        };

        let bundles = current_behavior.get_bundles(
            stage_time.elapsed(),
            position,
            speed.0,
            *continuous_depth,
        );
        match bundles {
            BehaviorBundle::Idle | BehaviorBundle::Attack => {}
            BehaviorBundle::Jump(jump_movement) => {
                // Spawn tween children to drive the jump arc movement.
                current_behavior.spawn_tween_children(
                    &mut commands,
                    entity,
                    position,
                    speed.0,
                    *continuous_depth,
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
                    *continuous_depth,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stage::{
        components::placement::{Depth, Speed},
        enemy::data::steps::{EnemyStep, IdleEnemyStep},
    };
    use std::{collections::VecDeque, time::Duration};

    #[test]
    fn ensure_enemy_continuous_depth_is_available_same_update() {
        let mut app = App::new();
        app.insert_resource(Time::<StageTimeDomain>::default());
        app.add_systems(
            Update,
            (
                ensure_enemy_continuous_depth.before(check_no_behavior),
                check_no_behavior,
            ),
        );

        let entity = app
            .world_mut()
            .spawn((
                Enemy,
                EnemyBehaviors::new(VecDeque::from([EnemyStep::Idle(
                    IdleEnemyStep::base().with_duration(Duration::ZERO.as_secs_f32()),
                )])),
                WorldPos::default(),
                Speed(1.0),
                Depth::Three,
            ))
            .id();

        app.update();

        let entity_ref = app.world().entity(entity);
        assert!(
            (entity_ref
                .get::<EnemyContinuousDepth>()
                .expect("continuous depth should be seeded immediately")
                .0
                - Depth::Three.to_f32())
            .abs()
                < f32::EPSILON
        );
        assert!(
            entity_ref.get::<EnemyCurrentBehavior>().is_some(),
            "behavior assignment should still happen on the same update"
        );
    }
}
