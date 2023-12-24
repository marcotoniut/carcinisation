use super::components::*;
use crate::{core::time::DeltaTime, plugins::movement::structs::MovementVec2Position};
use bevy::prelude::*;

/** TODO generalise current position via a generic LinearPosition trait */
pub fn update<T: DeltaTime + 'static + Resource, P: MovementVec2Position + Component>(
    mut movement_query: Query<
        (&mut P, &PursueSpeed<T, P>, &PursueTargetPosition<T, P>),
        Without<PursueTargetReached<T, P>>,
    >,
    delta_time: Res<T>,
) {
    for (mut position, speed, target) in &mut movement_query.iter_mut() {
        let direction = target.value - position.get();
        let direction = direction.normalize_or_zero();
        let direction = direction * speed.value * delta_time.delta_seconds();

        position.add(direction);
    }
}

/**
 * What to do if there's already a bundle? Should I simply clean it up on added?
 */
pub fn on_position_added<T: DeltaTime + 'static + Resource, P: MovementVec2Position + Component>(
    mut commands: Commands,
    mut movement_query: Query<(Entity, Added<PursueTargetPosition<T, P>>)>,
) {
    for (entity, _) in &mut movement_query.iter_mut() {
        commands
            .entity(entity)
            .remove::<PursueTargetXReached<T, P>>()
            .remove::<PursueTargetYReached<T, P>>()
            .remove::<PursueTargetReached<T, P>>();
    }
}

pub fn check_x_reached<T: DeltaTime + 'static + Resource, P: MovementVec2Position + Component>(
    mut commands: Commands,
    mut movement_query: Query<
        (Entity, &P, &PursueSpeed<T, P>, &PursueTargetPosition<T, P>),
        Without<PursueTargetXReached<T, P>>,
    >,
) {
    for (entity, position, speed, target) in &mut movement_query.iter_mut() {
        let vec = position.get();
        if (speed.value.x < 0. && vec.x <= target.value.x)
            || (speed.value.x > 0. && vec.x >= target.value.x)
        {
            commands
                .entity(entity)
                .insert(PursueTargetXReached::<T, P>::new());
        }
    }
}

pub fn check_y_reached<T: DeltaTime + 'static + Resource, P: MovementVec2Position + Component>(
    mut commands: Commands,
    mut movement_query: Query<
        (Entity, &P, &PursueSpeed<T, P>, &PursueTargetPosition<T, P>),
        Without<PursueTargetYReached<T, P>>,
    >,
) {
    for (entity, position, speed, target) in &mut movement_query.iter_mut() {
        let vec = position.get();
        if (speed.value.y < 0. && vec.y <= target.value.y)
            || (speed.value.y > 0. && vec.y >= target.value.y)
        {
            commands
                .entity(entity)
                .insert(PursueTargetYReached::<T, P>::new());
        }
    }
}

// TODO, could be using the other checks at the same time to avoid a next frame "Reached" status
pub fn check_reached<T: DeltaTime + 'static + Resource, P: MovementVec2Position>(
    mut commands: Commands,
    mut movement_query: Query<
        Entity,
        (
            With<PursueTargetXReached<T, P>>,
            With<PursueTargetYReached<T, P>>,
            Without<PursueTargetReached<T, P>>,
        ),
    >,
) {
    for entity in &mut movement_query.iter_mut() {
        {
            commands
                .entity(entity)
                .insert(PursueTargetReached::<T, P>::new());
        }
    }
}
