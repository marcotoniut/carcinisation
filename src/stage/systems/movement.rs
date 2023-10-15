use bevy::prelude::*;
use seldom_pixel::prelude::PxSubPosition;

use crate::{
    plugins::movement::{linear::components::*, structs::MovementDirection},
    stage::{
        components::placement::{Depth, LinearUpdateDisabled},
        enemy::components::{behavior::EnemyCurrentBehavior, CircleAround, LinearMovement},
        events::DepthChangedEvent,
        resources::StageTime,
    },
};

pub fn update_position_x(
    mut query: Query<
        (&TargetingPositionX, &mut PxSubPosition),
        (
            Without<LinearUpdateDisabled>,
            Without<LinearTargetReached<StageTime, TargetingPositionX>>,
        ),
    >,
) {
    for (progress, mut position) in &mut query.iter_mut() {
        position.0.x = progress.0;
    }
}

pub fn update_position_y(
    mut query: Query<
        (&TargetingPositionY, &mut PxSubPosition),
        (
            Without<LinearUpdateDisabled>,
            Without<LinearTargetReached<StageTime, TargetingPositionY>>,
        ),
    >,
) {
    for (progress, mut position) in &mut query.iter_mut() {
        position.0.y = progress.0;
    }
}

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
    mut event_writer: EventWriter<DepthChangedEvent>,
) {
    for (entity, mut depth, position, speed) in &mut query.iter_mut() {
        if speed.value > 0.0 {
            let next_depth = depth.0 + 1;
            if position.0 >= (depth.0 as f32 + 0.5) {
                depth.0 = next_depth;
                // REVIEW should this use DepthChanged, or Added<LinearTargetReached> (LinearTargetReached<StageTime, ZAxisPosition>)
                event_writer.send(DepthChangedEvent::new(entity, depth.clone()));
            }
        } else {
            let next_depth = depth.0 - 1;
            if position.0 <= (depth.0 as f32 - 0.5) {
                depth.0 = next_depth;
                event_writer.send(DepthChangedEvent::new(entity, depth.clone()));
            }
        }
    }
}

pub fn circle_around(time: Res<Time>, mut query: Query<(&CircleAround, &mut PxSubPosition)>) {
    for (circle_around, mut position) in query.iter_mut() {
        let elapsed_seconds = time.elapsed_seconds();
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
