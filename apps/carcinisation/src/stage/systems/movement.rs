use crate::stage::{
    components::placement::Depth,
    enemy::{
        components::{
            CircleAround, LinearTween,
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

/// @system Recalculates entity depth from the Z tween value and emits `DepthChangedMessage`.
pub fn update_depth(
    mut query: Query<
        (Entity, &mut Depth, &TargetingValueZ),
        (
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
        let mut depth_f32 = depth.to_f32();

        // Handle moving deeper
        while position.0 >= (depth_f32 + 0.5) {
            *depth = *depth + 1;
            depth_f32 = depth.to_f32();
            event_writer.write(DepthChangedMessage::new(entity, *depth));
        }

        // Handle moving shallower
        while position.0 <= (depth_f32 - 0.5) {
            *depth = *depth - 1;
            depth_f32 = depth.to_f32();
            event_writer.write(DepthChangedMessage::new(entity, *depth));
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
}
