use bevy::prelude::*;
use seldom_pixel::prelude::PxSubPosition;

use crate::stage::{
    components::{Depth, DepthProgress, DepthReached, DepthSpeed, TargetDepth},
    data::MovementDirection,
    enemy::components::CircleAround,
    events::DepthChanged,
};

pub fn advance_incoming(
    mut incoming_query: Query<(&DepthSpeed, &mut DepthProgress), Without<DepthReached>>,
    time: Res<Time>,
) {
    for (speed, mut depth) in &mut incoming_query.iter_mut() {
        depth.0 += speed.0 * time.delta_seconds();
    }
}

pub fn update_depth(
    mut incoming_query: Query<
        (Entity, &mut Depth, &DepthProgress, &DepthSpeed),
        Without<DepthReached>,
    >,
    mut event_writer: EventWriter<DepthChanged>,
) {
    for (entity, mut depth, progress, speed) in &mut incoming_query.iter_mut() {
        if speed.0 > 0.0 {
            let next_depth = depth.0 + 1;
            if progress.0 >= (depth.0 as f32 + 0.5) {
                depth.0 = next_depth;
                event_writer.send(DepthChanged {
                    entity,
                    depth: depth.clone(),
                });
            }
        } else {
            let next_depth = depth.0 - 1;
            if progress.0 <= (depth.0 as f32 - 0.5) {
                depth.0 = next_depth;
                event_writer.send(DepthChanged {
                    entity,
                    depth: depth.clone(),
                });
            }
        }
    }
}

pub fn check_depth_reached(
    mut commands: Commands,
    mut incoming_query: Query<(Entity, &Depth, &TargetDepth), Without<DepthReached>>,
) {
    for (entity, depth, target) in &mut incoming_query.iter_mut() {
        if depth.0 == target.0 {
            commands.entity(entity).insert(DepthReached {});
        }
    }
}

pub fn circle_around(time: Res<Time>, mut query: Query<(&CircleAround, &mut PxSubPosition)>) {
    for (circle_around, mut position) in query.iter_mut() {
        let elapsed_seconds = time.elapsed_seconds();
        let angle = match circle_around.direction {
            MovementDirection::Right => elapsed_seconds + circle_around.time_offset,
            MovementDirection::Left => -elapsed_seconds + circle_around.time_offset,
        };
        let x = circle_around.center.x + circle_around.radius * angle.cos();
        let y = circle_around.center.y + circle_around.radius * angle.sin();
        position.0 = Vec2::new(x, y);
    }
}
