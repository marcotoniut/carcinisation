use crate::stage::{
    components::placement::Depth,
    enemy::{
        components::{
            CircleAround, Enemy, EnemyContinuousDepth, LinearTween,
            behavior::{EnemyCurrentBehavior, EnemyStepTweenChild, JumpTween},
        },
        mosquiton::entity::WingsBroken,
    },
    messages::DepthChangedMessage,
    resources::StageTimeDomain,
};
use bevy::{ecs::hierarchy::ChildOf, prelude::*};
use carapace::prelude::WorldPos;
use cween::{
    linear::components::{LinearValueReached, TargetingValueX, TargetingValueY, TargetingValueZ},
    structs::TweenDirection,
};

/// Keeps enemy continuous depth in sync with tween-driven Z motion.
pub fn sync_enemy_continuous_depth_from_targeting_z(
    mut query: Query<
        (&mut EnemyContinuousDepth, &TargetingValueZ),
        (
            With<Enemy>,
            Or<(Added<TargetingValueZ>, Changed<TargetingValueZ>)>,
        ),
    >,
) {
    for (mut continuous_depth, targeting_depth) in &mut query {
        let new_depth = Depth::clamp_continuous(targeting_depth.0);
        if (continuous_depth.0 - new_depth).abs() >= f32::EPSILON {
            continuous_depth.0 = new_depth;
        }
    }
}

/// Derives the gameplay depth bucket from canonical continuous enemy depth.
pub fn derive_enemy_depth_from_continuous(
    mut query: Query<
        (Entity, &EnemyContinuousDepth, &mut Depth),
        (
            With<Enemy>,
            Or<(Added<EnemyContinuousDepth>, Changed<EnemyContinuousDepth>)>,
        ),
    >,
    mut event_writer: MessageWriter<DepthChangedMessage>,
) {
    for (entity, continuous_depth, mut depth) in &mut query {
        let new_depth = continuous_depth.snapped_depth();
        if *depth != new_depth {
            *depth = new_depth;
            event_writer.write(DepthChangedMessage::new(entity, new_depth));
        }
    }
}

/// Preserves the previous discrete-depth tween behaviour for non-enemy entities
/// that still express depth only through `TargetingValueZ`.
pub fn update_non_enemy_depth_from_targeting_z(
    mut query: Query<
        (Entity, &mut Depth, &TargetingValueZ),
        (
            Without<Enemy>,
            Without<LinearValueReached<StageTimeDomain, TargetingValueZ>>,
            Or<(
                Added<TargetingValueZ>,
                Changed<TargetingValueZ>,
                Changed<Depth>,
            )>,
        ),
    >,
    mut event_writer: MessageWriter<DepthChangedMessage>,
) {
    for (entity, mut depth, position) in &mut query.iter_mut() {
        let new_depth = Depth::from_continuous(position.0);
        if *depth != new_depth {
            *depth = new_depth;
            event_writer.write(DepthChangedMessage::new(entity, new_depth));
        }
    }
}

/// @system Orbits entities around a centre point using elapsed time.
///
/// `Without<WingsBroken>` is a defensive backstop: the primary cleanup
/// removes `CircleAround` at the wing-break transition point in
/// `detect_part_breakage`, but this filter prevents orbit writes if that
/// cleanup ever regresses.
pub fn circle_around(
    time: Res<Time<StageTimeDomain>>,
    mut query: Query<(&CircleAround, &mut WorldPos), Without<WingsBroken>>,
) {
    for (circle_around, mut position) in &mut query {
        let elapsed_seconds = time.elapsed().as_secs_f32();
        let angle = match circle_around.direction {
            TweenDirection::Positive => elapsed_seconds + circle_around.time_offset,
            TweenDirection::Negative => -elapsed_seconds + circle_around.time_offset,
        };
        let x = circle_around.center.x + circle_around.radius * angle.cos();
        let y = circle_around.center.y + circle_around.radius * angle.sin();
        position.0 = Vec2::new(x, y);
    }
}

/// @system Detects when enemy X-axis tween children reach their target.
/// Updates the parent enemy's `LinearTween.reached_x` flag.
pub fn check_linear_tween_x_finished(
    mut parent_query: Query<&mut LinearTween, With<EnemyCurrentBehavior>>,
    mut jump_query: Query<&mut JumpTween, With<EnemyCurrentBehavior>>,
    child_query: Query<
        &ChildOf,
        (
            With<EnemyStepTweenChild>,
            Added<LinearValueReached<StageTimeDomain, TargetingValueX>>,
        ),
    >,
) {
    for child_of in child_query.iter() {
        if let Ok(mut linear_movement) = parent_query.get_mut(child_of.0) {
            linear_movement.reached_x = true;
        }
        if let Ok(mut jump_movement) = jump_query.get_mut(child_of.0) {
            jump_movement.reached_x = true;
        }
    }
}

/// @system Detects when enemy Y-axis tween children reach their target.
/// Updates the parent enemy's `LinearTween.reached_y` flag.
pub fn check_linear_tween_y_finished(
    mut parent_query: Query<&mut LinearTween, With<EnemyCurrentBehavior>>,
    mut jump_query: Query<&mut JumpTween, With<EnemyCurrentBehavior>>,
    child_query: Query<
        &ChildOf,
        (
            With<EnemyStepTweenChild>,
            Added<LinearValueReached<StageTimeDomain, TargetingValueY>>,
        ),
    >,
) {
    for child_of in child_query.iter() {
        if let Ok(mut linear_movement) = parent_query.get_mut(child_of.0) {
            linear_movement.reached_y = true;
        }
        if let Ok(mut jump_movement) = jump_query.get_mut(child_of.0) {
            jump_movement.reached_y = true;
        }
    }
}

/// Detects when enemy Z-axis jump tween children reach their target.
pub fn check_jump_tween_z_finished(
    mut parent_query: Query<&mut JumpTween, With<EnemyCurrentBehavior>>,
    child_query: Query<
        &ChildOf,
        (
            With<EnemyStepTweenChild>,
            Added<LinearValueReached<StageTimeDomain, TargetingValueZ>>,
        ),
    >,
) {
    for child_of in child_query.iter() {
        if let Ok(mut jump_movement) = parent_query.get_mut(child_of.0) {
            jump_movement.reached_z = true;
        }
    }
}

/// @system Removes `EnemyCurrentBehavior` once both X and Y tweens are done.
// TODO this should not be tied to the stage tween
pub fn check_linear_tween_finished(
    mut commands: Commands,
    query: Query<(Entity, &LinearTween), (With<EnemyCurrentBehavior>,)>,
) {
    for (entity, linear_movement) in query {
        if linear_movement.reached_x && linear_movement.reached_y {
            commands
                .entity(entity)
                .remove::<EnemyCurrentBehavior>()
                .remove::<LinearTween>();
        }
    }
}

/// Removes `EnemyCurrentBehavior` once all jump tween axes are done.
pub fn check_jump_tween_finished(
    mut commands: Commands,
    query: Query<(Entity, &JumpTween), With<EnemyCurrentBehavior>>,
) {
    for (entity, jump_movement) in query {
        if jump_movement.is_finished() {
            commands
                .entity(entity)
                .remove::<EnemyCurrentBehavior>()
                .remove::<JumpTween>();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stage::enemy::data::steps::{EnemyStep, JumpEnemyStep};
    use std::time::Duration;

    #[test]
    fn jump_tween_completion_clears_current_behavior() {
        let mut app = App::new();
        app.add_systems(Update, check_jump_tween_finished);

        let entity = app
            .world_mut()
            .spawn((
                EnemyCurrentBehavior {
                    started: Duration::ZERO,
                    behavior: EnemyStep::Jump(JumpEnemyStep::base()),
                },
                JumpTween {
                    started: Duration::ZERO,
                    travel_time_secs: 1.0,
                    reached_x: true,
                    reached_y: true,
                    reached_z: true,
                    expects_z: false,
                },
            ))
            .id();

        app.update();

        let entity_ref = app.world().entity(entity);
        assert!(entity_ref.get::<EnemyCurrentBehavior>().is_none());
        assert!(entity_ref.get::<JumpTween>().is_none());
    }

    #[test]
    fn enemy_continuous_depth_derives_discrete_bucket() {
        let mut app = App::new();
        app.add_message::<DepthChangedMessage>();
        app.add_systems(Update, derive_enemy_depth_from_continuous);

        let entity = app
            .world_mut()
            .spawn((Enemy, EnemyContinuousDepth(4.6), Depth::Three))
            .id();

        app.update();

        assert_eq!(
            *app.world()
                .entity(entity)
                .get::<Depth>()
                .expect("enemy should keep depth bucket"),
            Depth::Five
        );
    }

    #[test]
    fn enemy_targeting_z_updates_continuous_depth() {
        let mut app = App::new();
        app.add_systems(Update, sync_enemy_continuous_depth_from_targeting_z);

        let entity = app
            .world_mut()
            .spawn((
                Enemy,
                EnemyContinuousDepth::from_depth(Depth::Three),
                TargetingValueZ::new(5.25),
            ))
            .id();

        app.update();

        assert!(
            (app.world()
                .entity(entity)
                .get::<EnemyContinuousDepth>()
                .expect("enemy should keep continuous depth")
                .0
                - 5.25)
                .abs()
                < f32::EPSILON
        );
    }
}
