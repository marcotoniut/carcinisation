use crate::{
    plugins::movement::{linear::components::*, structs::MovementDirection},
    stage::{
        components::placement::Depth,
        enemy::components::{
            behavior::{EnemyCurrentBehavior, EnemyStepMovement},
            CircleAround, LinearMovement,
        },
        events::DepthChangedEvent,
        resources::StageTimeDomain,
    },
};
use bevy::{ecs::hierarchy::ChildOf, prelude::*};
use seldom_pixel::prelude::PxSubPosition;

pub fn update_depth(
    mut query: Query<
        (Entity, &mut Depth, &TargetingPositionZ),
        (
            Without<LinearTargetReached<StageTimeDomain, TargetingPositionZ>>,
            Or<(
                Added<TargetingPositionZ>,
                Changed<TargetingPositionZ>,
                Changed<Depth>,
            )>,
        ),
    >,
    mut event_writer: MessageWriter<DepthChangedEvent>,
) {
    for (entity, mut depth, position) in &mut query.iter_mut() {
        let mut depth_f32 = depth.to_f32();

        // Handle moving deeper
        while position.0 >= (depth_f32 + 0.5) {
            *depth = *depth + 1;
            depth_f32 = depth.to_f32();
            event_writer.write(DepthChangedEvent::new(entity, *depth));
        }

        // Handle moving shallower
        while position.0 <= (depth_f32 - 0.5) {
            *depth = *depth - 1;
            depth_f32 = depth.to_f32();
            event_writer.write(DepthChangedEvent::new(entity, *depth));
        }
    }
}

pub fn circle_around(
    time: Res<Time<StageTimeDomain>>,
    mut query: Query<(&CircleAround, &mut PxSubPosition)>,
) {
    for (circle_around, mut position) in query.iter_mut() {
        let elapsed_seconds = time.elapsed().as_secs_f32();
        let angle = match circle_around.direction {
            MovementDirection::Positive => elapsed_seconds + circle_around.time_offset,
            MovementDirection::Negative => -elapsed_seconds + circle_around.time_offset,
        };
        let x = circle_around.center.x + circle_around.radius * angle.cos();
        let y = circle_around.center.y + circle_around.radius * angle.sin();
        position.0 = Vec2::new(x, y);
    }
}

/// @system Detects when enemy X-axis movement children reach their target.
/// Updates the parent enemy's LinearMovement.reached_x flag.
pub fn check_linear_movement_x_finished(
    mut parent_query: Query<&mut LinearMovement, With<EnemyCurrentBehavior>>,
    child_query: Query<
        &ChildOf,
        (
            With<EnemyStepMovement>,
            Added<LinearTargetReached<StageTimeDomain, TargetingPositionX>>,
        ),
    >,
) {
    for child_of in child_query.iter() {
        if let Ok(mut linear_movement) = parent_query.get_mut(child_of.0) {
            linear_movement.reached_x = true;
        }
    }
}

/// @system Detects when enemy Y-axis movement children reach their target.
/// Updates the parent enemy's LinearMovement.reached_y flag.
pub fn check_linear_movement_y_finished(
    mut parent_query: Query<&mut LinearMovement, With<EnemyCurrentBehavior>>,
    child_query: Query<
        &ChildOf,
        (
            With<EnemyStepMovement>,
            Added<LinearTargetReached<StageTimeDomain, TargetingPositionY>>,
        ),
    >,
) {
    for child_of in child_query.iter() {
        if let Ok(mut linear_movement) = parent_query.get_mut(child_of.0) {
            linear_movement.reached_y = true;
        }
    }
}

// TODO this should not be tied to the stage movement
pub fn check_linear_movement_finished(
    mut commands: Commands,
    mut query: Query<(Entity, &LinearMovement), (With<EnemyCurrentBehavior>,)>,
) {
    for (entity, linear_movement) in query.iter_mut() {
        if linear_movement.reached_x && linear_movement.reached_y {
            commands
                .entity(entity)
                .remove::<EnemyCurrentBehavior>()
                .remove::<LinearMovement>();
        }
    }
}
