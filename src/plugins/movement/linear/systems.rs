use bevy::prelude::*;

use super::components::*;

/** TODO generalise current position via a generic LinearPosition trait */
pub fn move_linear<T: DeltaTime + 'static + Resource, P: LinearPosition + 'static + Component>(
    mut movement_query: Query<
        (&mut P, &LinearSpeed<T>, &LinearTargetPosition<T>),
        Without<LinearTargetReached<T>>,
    >,
    delta_time: Res<T>,
) {
    for (mut position, speed, target) in &mut movement_query.iter_mut() {
        let direction = target.value - position.get();
        let direction = direction.normalize();
        let direction = direction * speed.value * delta_time.delta_seconds();

        position.add(direction);
    }
}

pub fn check_linear_x_reached<
    T: DeltaTime + 'static + Resource,
    P: LinearPosition + 'static + Component,
>(
    mut commands: Commands,
    mut movement_query: Query<
        (Entity, &P, &LinearSpeed<T>, &LinearTargetPosition<T>),
        Without<LinearTargetXReached<T>>,
    >,
) {
    for (entity, position, speed, target) in &mut movement_query.iter_mut() {
        let vec = position.get();
        if (speed.value.x < 0. && vec.x > target.value.x)
            || (speed.value.x > 0. && vec.x > target.value.x)
        {
            commands
                .entity(entity)
                .insert(LinearTargetXReached::<T>::new());
        }
    }
}

pub fn check_linear_y_reached<
    T: DeltaTime + 'static + Resource,
    P: LinearPosition + 'static + Component,
>(
    mut commands: Commands,
    mut movement_query: Query<
        (Entity, &P, &LinearSpeed<T>, &LinearTargetPosition<T>),
        Without<LinearTargetYReached<T>>,
    >,
) {
    for (entity, position, speed, target) in &mut movement_query.iter_mut() {
        let vec = position.get();
        if (speed.value.y < 0. && vec.y > target.value.y)
            || (speed.value.y > 0. && vec.y > target.value.y)
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
