use crate::{
    plugins::movement::{linear::components::*, structs::MovementDirection},
    stage::{
        components::placement::Depth,
        enemy::components::{behavior::EnemyCurrentBehavior, CircleAround, LinearMovement},
        events::DepthChangedEvent,
        resources::StageTime,
    },
};
use bevy::prelude::*;
use seldom_pixel::prelude::PxSubPosition;

pub fn update_depth(
    mut query: Query<
        (
            Entity,
            &mut Depth,
            &TargetingPositionZ,
            &LinearSpeed<StageTime, TargetingPositionZ>,
        ),
        Without<LinearTargetReached<StageTime, TargetingPositionZ>>,
    >,
    mut event_writer: MessageWriter<DepthChangedEvent>,
) {
    for (entity, mut depth, position, speed) in &mut query.iter_mut() {
        if speed.value > 0.0 {
            if position.0 >= (depth.to_f32() + 0.5) {
                *depth = *depth + 1;
                // REVIEW should this use DepthChanged, or Added<LinearTargetReached> (LinearTargetReached<StageTime, ZAxisPosition>)
                event_writer.write(DepthChangedEvent::new(entity, *depth));
            }
        } else if position.0 <= (depth.to_f32() - 0.5) {
            *depth = *depth - 1;
            event_writer.write(DepthChangedEvent::new(entity, *depth));
        }
    }
}

pub fn circle_around(time: Res<Time>, mut query: Query<(&CircleAround, &mut PxSubPosition)>) {
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

pub fn check_linear_movement_x_finished(
    mut query: Query<
        &mut LinearMovement,
        (
            With<EnemyCurrentBehavior>,
            Added<LinearTargetReached<StageTime, TargetingPositionX>>,
        ),
    >,
) {
    for mut linear_movement in query.iter_mut() {
        linear_movement.reached_x = true;
    }
}

pub fn check_linear_movement_y_finished(
    mut query: Query<
        &mut LinearMovement,
        (
            With<EnemyCurrentBehavior>,
            Added<LinearTargetReached<StageTime, TargetingPositionY>>,
        ),
    >,
) {
    for mut linear_movement in query.iter_mut() {
        linear_movement.reached_y = true;
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
