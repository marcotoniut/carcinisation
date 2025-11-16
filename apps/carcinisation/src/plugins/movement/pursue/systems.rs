#![allow(clippy::type_complexity)]

use super::components::*;
use crate::plugins::movement::structs::MovementVec2Position;
use bevy::{ecs::component::Mutable, prelude::*};

/** TODO generalise current position via a generic LinearPosition trait */
pub fn update<D, P>(
    mut movement_query: Query<
        (&mut P, &PursueSpeed<D, P>, &PursueTargetPosition<D, P>),
        Without<PursueTargetReached<D, P>>,
    >,
    delta_time: Res<Time<D>>,
) where
    D: Default + Send + Sync + 'static,
    P: MovementVec2Position + Component<Mutability = Mutable>,
{
    for (mut position, speed, target) in movement_query.iter_mut() {
        let direction = target.value - position.get();
        let direction = direction.normalize_or_zero();
        let direction = direction * speed.value * delta_time.delta().as_secs_f32();

        position.add(direction);
    }
}

/**
 * What to do if there's already a bundle? Should I simply clean it up on added?
 */
pub fn on_position_added<D, P>(
    mut commands: Commands,
    movement_query: Query<Entity, Added<PursueTargetPosition<D, P>>>,
) where
    D: Default + Send + Sync + 'static,
    P: MovementVec2Position + Component,
{
    for entity in movement_query.iter() {
        commands
            .entity(entity)
            .remove::<PursueTargetXReached<D, P>>()
            .remove::<PursueTargetYReached<D, P>>()
            .remove::<PursueTargetReached<D, P>>();
    }
}

pub fn check_x_reached<D, P>(
    mut commands: Commands,
    mut movement_query: Query<
        (Entity, &P, &PursueSpeed<D, P>, &PursueTargetPosition<D, P>),
        Without<PursueTargetXReached<D, P>>,
    >,
) where
    D: Default + Send + Sync + 'static,
    P: MovementVec2Position + Component,
{
    for (entity, position, speed, target) in movement_query.iter_mut() {
        let vec = position.get();
        if (speed.value.x < 0. && vec.x <= target.value.x)
            || (speed.value.x > 0. && vec.x >= target.value.x)
        {
            commands
                .entity(entity)
                .insert(PursueTargetXReached::<D, P>::new());
        }
    }
}

pub fn check_y_reached<D, P>(
    mut commands: Commands,
    mut movement_query: Query<
        (Entity, &P, &PursueSpeed<D, P>, &PursueTargetPosition<D, P>),
        Without<PursueTargetYReached<D, P>>,
    >,
) where
    D: Default + Send + Sync + 'static,
    P: MovementVec2Position + Component,
{
    for (entity, position, speed, target) in movement_query.iter_mut() {
        let vec = position.get();
        if (speed.value.y < 0. && vec.y <= target.value.y)
            || (speed.value.y > 0. && vec.y >= target.value.y)
        {
            commands
                .entity(entity)
                .insert(PursueTargetYReached::<D, P>::new());
        }
    }
}

// TODO, could be using the other checks at the same time to avoid a next frame "Reached" status
pub fn check_reached<D, P>(
    mut commands: Commands,
    movement_query: Query<
        Entity,
        (
            With<PursueTargetXReached<D, P>>,
            With<PursueTargetYReached<D, P>>,
            Without<PursueTargetReached<D, P>>,
        ),
    >,
) where
    D: Default + Send + Sync + 'static,
    P: MovementVec2Position,
{
    for entity in movement_query.iter() {
        {
            commands
                .entity(entity)
                .insert(PursueTargetReached::<D, P>::new());
        }
    }
}
