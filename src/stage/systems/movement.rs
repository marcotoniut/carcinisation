use bevy::prelude::*;
use seldom_pixel::prelude::PxSubPosition;

use crate::stage::components::{
    Depth, DepthReached, IncomingSpeed, LineSpeed, TargetDepth, TargetPosition, TargetReached,
    TargetXReached, TargetYReached,
};

pub fn advance_incoming(mut incoming_query: Query<(&IncomingSpeed, &mut Depth)>, time: Res<Time>) {
    for (speed, mut depth) in &mut incoming_query.iter_mut() {
        depth.0 += speed.0 * time.delta_seconds();
    }
}

pub fn check_depth_reached(
    mut commands: Commands,
    mut incoming_query: Query<(Entity, &Depth, &TargetDepth), Without<DepthReached>>,
) {
    for (entity, depth, target) in &mut incoming_query.iter_mut() {
        if depth.0 >= target.0 {
            commands.entity(entity).insert(DepthReached {});
        }
    }
}

pub fn advance_line(
    mut movement_query: Query<(&mut PxSubPosition, &LineSpeed, &TargetPosition)>,
    time: Res<Time>,
) {
    for (mut position, speed, target) in &mut movement_query.iter_mut() {
        let direction = target.0 - position.0;
        let direction = direction.normalize();
        let direction = direction * speed.0 * time.delta_seconds();

        position.0 += direction;
    }
}

pub fn check_line_target_x_reached(
    mut commands: Commands,
    mut movement_query: Query<
        (Entity, &PxSubPosition, &LineSpeed, &TargetPosition),
        Without<TargetXReached>,
    >,
) {
    for (entity, position, speed, target) in &mut movement_query.iter_mut() {
        if (speed.0.x < 0. && position.0.x > target.0.x)
            || (speed.0.x > 0. && position.0.x > target.0.x)
        {
            commands.entity(entity).insert(TargetXReached {});
        }
    }
}

pub fn check_line_target_y_reached(
    mut commands: Commands,
    mut movement_query: Query<
        (Entity, &PxSubPosition, &LineSpeed, &TargetPosition),
        Without<TargetYReached>,
    >,
) {
    for (entity, position, speed, target) in &mut movement_query.iter_mut() {
        if (speed.0.y < 0. && position.0.y > target.0.y)
            || (speed.0.y > 0. && position.0.y > target.0.y)
        {
            commands.entity(entity).insert(TargetYReached {});
        }
    }
}

// With, Without
pub fn check_line_target_reached(
    mut commands: Commands,
    mut movement_query: Query<(Entity, &TargetXReached, &TargetYReached), Without<TargetReached>>,
) {
    for (entity, _, _) in &mut movement_query.iter_mut() {
        {
            commands.entity(entity).insert(TargetReached {});
        }
    }
}
