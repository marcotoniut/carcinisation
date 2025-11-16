#![allow(clippy::type_complexity)]

use bevy::{ecs::component::Mutable, prelude::*};

use crate::plugins::movement::structs::{Magnitude, MovementDirection};

use super::components::{extra::LinearMovement2DReachCheck, *};

pub fn update<D, P>(
    mut query: Query<
        (
            &mut P,
            &mut LinearSpeed<D, P>,
            Option<&LinearAcceleration<D, P>>,
        ),
        Without<LinearTargetReached<D, P>>,
    >,
    delta_time: Res<Time<D>>,
) where
    D: Default + Send + Sync + 'static,
    P: Magnitude + Component<Mutability = Mutable>,
{
    for (mut position, mut speed, acceleration_o) in query.iter_mut() {
        if let Some(acceleration) = acceleration_o {
            speed.add(acceleration.value * delta_time.delta().as_secs_f32());
        }
        position.add(speed.value * delta_time.delta().as_secs_f32());
    }
}

/**
 * What to do if there's already a bundle? Should I simply clean it up on added?
 */
pub fn on_position_added<D, P>(
    mut commands: Commands,
    query: Query<Entity, Added<LinearTargetPosition<D, P>>>,
) where
    D: Default + Send + Sync + 'static,
    P: Magnitude + Component<Mutability = Mutable>,
{
    for entity in query.iter() {
        commands
            .entity(entity)
            .remove::<LinearTargetReached<D, P>>();
    }
}

pub fn check_reached<D, P>(
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &mut P,
            &LinearDirection<D, P>,
            &LinearTargetPosition<D, P>,
        ),
        Without<LinearTargetReached<D, P>>,
    >,
) where
    D: Default + Send + Sync + 'static,
    P: Magnitude + Component<Mutability = Mutable>,
{
    for (entity, mut position, direction, target) in query.iter_mut() {
        let x = position.get();
        if (direction.value == MovementDirection::Negative && x <= target.value)
            || (direction.value == MovementDirection::Positive && x >= target.value)
        {
            position.set(target.value);
            commands
                .entity(entity)
                .insert(LinearTargetReached::<D, P>::new());
        }
    }
}

// TODO check if this is prone to race conditions when systems that update data based on P execute in between this and update
pub fn on_reached<D, P>(
    mut commands: Commands,
    query: Query<Entity, Added<LinearTargetReached<D, P>>>,
) where
    D: Default + Send + Sync + 'static,
    P: Magnitude + Component<Mutability = Mutable>,
{
    for entity in query.iter() {
        commands
            .entity(entity)
            .remove::<LinearPositionRemovalBundle<D, P>>()
            .remove::<P>();
    }
}

pub fn check_2d_x_reached<D, X, Y>(
    mut query: Query<&mut LinearMovement2DReachCheck<D, X, Y>, Added<LinearTargetReached<D, X>>>,
) where
    D: Default + Send + Sync + 'static,
    X: Magnitude + Component,
    Y: Magnitude + Component,
{
    for mut check in query.iter_mut() {
        check.reached.0 = true;
    }
}

pub fn check_2d_y_reached<D, X, Y>(
    mut query: Query<&mut LinearMovement2DReachCheck<D, X, Y>, Added<LinearTargetReached<D, Y>>>,
) where
    D: Default + Send + Sync + 'static,
    X: Magnitude + Component,
    Y: Magnitude + Component,
{
    for mut check in query.iter_mut() {
        check.reached.1 = true;
    }
}
