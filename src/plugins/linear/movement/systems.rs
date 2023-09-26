use bevy::prelude::*;
use seldom_pixel::prelude::PxSubPosition;

use super::components::*;

/** TODO generalise current position via a generic LinearPosition trait */
pub fn move_linear<T: DeltaTime + 'static + Resource>(
    mut movement_query: Query<
        (
            &mut PxSubPosition,
            &LinearSpeed<T>,
            &LinearTargetPosition<T>,
        ),
        Without<LinearTargetReached<T>>,
    >,
    delta_time: Res<T>,
) {
    for (mut position, speed, target) in &mut movement_query.iter_mut() {
        let direction = target.value - position.0;
        let direction = direction.normalize();
        let normalize = direction.normalize();
        let seconds = delta_time.delta_seconds();
        let direction = direction * speed.value * seconds;
        let final_direction = direction * speed.value * seconds;

        position.0 += direction;
    }
}

pub fn check_linear_x_reached<T: DeltaTime + 'static + Resource>(
    mut commands: Commands,
    mut movement_query: Query<
        (
            Entity,
            &PxSubPosition,
            &LinearSpeed<T>,
            &LinearTargetPosition<T>,
        ),
        Without<LinearTargetXReached<T>>,
    >,
) {
    for (entity, position, speed, target) in &mut movement_query.iter_mut() {
        if (speed.value.x < 0. && position.0.x > target.value.x)
            || (speed.value.x > 0. && position.0.x > target.value.x)
        {
            commands
                .entity(entity)
                .insert(LinearTargetXReached::<T>::new());
        }
    }
}

pub fn check_linear_y_reached<T: DeltaTime + 'static + Resource>(
    mut commands: Commands,
    mut movement_query: Query<
        (
            Entity,
            &PxSubPosition,
            &LinearSpeed<T>,
            &LinearTargetPosition<T>,
        ),
        Without<LinearTargetYReached<T>>,
    >,
) {
    for (entity, position, speed, target) in &mut movement_query.iter_mut() {
        if (speed.value.y < 0. && position.0.y > target.value.y)
            || (speed.value.y > 0. && position.0.y > target.value.y)
        {
            commands
                .entity(entity)
                .insert(LinearTargetYReached::<T>::new());
        }
    }
}

// TODO, could be using the other checks at the same time to avoid a next frame "Reached" status
pub fn check_linear_reached<T: DeltaTime + 'static + Resource>(
    mut commands: Commands,
    mut movement_query: Query<
        Entity,
        (
            With<LinearTargetXReached<T>>,
            With<LinearTargetYReached<T>>,
            Without<LinearTargetReached<T>>,
        ),
    >,
) {
    for entity in &mut movement_query.iter_mut() {
        {
            commands
                .entity(entity)
                .insert(LinearTargetReached::<T>::new());
        }
    }
}
