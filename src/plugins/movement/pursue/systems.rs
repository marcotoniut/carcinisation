use bevy::prelude::*;

use super::components::*;

/** TODO generalise current position via a generic LinearPosition trait */
pub fn update<T: DeltaTime + 'static + Resource, P: Pursue + 'static + Component>(
    mut movement_query: Query<
        (&mut P, &PursueSpeed<T>, &PursueTargetPosition<T>),
        Without<PursueTargetReached<T>>,
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

pub fn check_x_reached<T: DeltaTime + 'static + Resource, P: Pursue + 'static + Component>(
    mut commands: Commands,
    mut movement_query: Query<
        (Entity, &P, &PursueSpeed<T>, &PursueTargetPosition<T>),
        Without<PursueTargetXReached<T>>,
    >,
) {
    for (entity, position, speed, target) in &mut movement_query.iter_mut() {
        let vec = position.get();
        if (speed.value.x < 0. && vec.x <= target.value.x)
            || (speed.value.x > 0. && vec.x >= target.value.x)
        {
            commands
                .entity(entity)
                .insert(PursueTargetXReached::<T>::new());
        }
    }
}

pub fn check_y_reached<T: DeltaTime + 'static + Resource, P: Pursue + 'static + Component>(
    mut commands: Commands,
    mut movement_query: Query<
        (Entity, &P, &PursueSpeed<T>, &PursueTargetPosition<T>),
        Without<PursueTargetYReached<T>>,
    >,
) {
    for (entity, position, speed, target) in &mut movement_query.iter_mut() {
        let vec = position.get();
        if (speed.value.y < 0. && vec.y <= target.value.y)
            || (speed.value.y > 0. && vec.y >= target.value.y)
        {
            commands
                .entity(entity)
                .insert(PursueTargetYReached::<T>::new());
        }
    }
}

// TODO, could be using the other checks at the same time to avoid a next frame "Reached" status
pub fn check_reached<T: DeltaTime + 'static + Resource>(
    mut commands: Commands,
    mut movement_query: Query<
        Entity,
        (
            With<PursueTargetXReached<T>>,
            With<PursueTargetYReached<T>>,
            Without<PursueTargetReached<T>>,
        ),
    >,
) {
    for entity in &mut movement_query.iter_mut() {
        {
            commands
                .entity(entity)
                .insert(PursueTargetReached::<T>::new());
        }
    }
}
