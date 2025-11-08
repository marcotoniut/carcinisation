#![allow(clippy::type_complexity)]

use bevy::{ecs::component::Mutable, prelude::*};

use crate::{
    core::time::DeltaTime,
    plugins::movement::structs::{Magnitude, MovementDirection},
};

use super::components::{extra::LinearMovement2DReachCheck, *};

pub fn update<T: DeltaTime + 'static + Resource, P: Magnitude + Component<Mutability = Mutable>>(
    mut query: Query<
        (
            &mut P,
            &mut LinearSpeed<T, P>,
            Option<&LinearAcceleration<T, P>>,
        ),
        Without<LinearTargetReached<T, P>>,
    >,
    delta_time: Res<T>,
) {
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
pub fn on_position_added<
    T: DeltaTime + 'static + Resource,
    P: Magnitude + Component<Mutability = Mutable>,
>(
    mut commands: Commands,
    query: Query<Entity, Added<LinearTargetPosition<T, P>>>,
) {
    for entity in query.iter() {
        commands
            .entity(entity)
            .remove::<LinearTargetReached<T, P>>();
    }
}

pub fn check_reached<
    T: DeltaTime + 'static + Resource,
    P: Magnitude + Component<Mutability = Mutable>,
>(
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &mut P,
            &LinearDirection<T, P>,
            &LinearTargetPosition<T, P>,
        ),
        Without<LinearTargetReached<T, P>>,
    >,
) {
    for (entity, mut position, direction, target) in query.iter_mut() {
        let x = position.get();
        if (direction.value == MovementDirection::Negative && x <= target.value)
            || (direction.value == MovementDirection::Positive && x >= target.value)
        {
            position.set(target.value);
            commands
                .entity(entity)
                .insert(LinearTargetReached::<T, P>::new());
        }
    }
}

// TODO check if this is prone to race conditions when systems that update data based on P execute in between this and update
pub fn on_reached<
    T: DeltaTime + 'static + Resource,
    P: Magnitude + Component<Mutability = Mutable>,
>(
    mut commands: Commands,
    query: Query<Entity, Added<LinearTargetReached<T, P>>>,
) {
    for entity in query.iter() {
        commands
            .entity(entity)
            .remove::<LinearPositionRemovalBundle<T, P>>()
            .remove::<P>();
    }
}

pub fn check_2d_x_reached<
    T: DeltaTime + 'static + Resource,
    X: Magnitude + Component,
    Y: Magnitude + Component,
>(
    mut query: Query<&mut LinearMovement2DReachCheck<T, X, Y>, Added<LinearTargetReached<T, X>>>,
) {
    for mut check in query.iter_mut() {
        check.reached.0 = true;
    }
}

pub fn check_2d_y_reached<
    T: DeltaTime + 'static + Resource,
    X: Magnitude + Component,
    Y: Magnitude + Component,
>(
    mut query: Query<&mut LinearMovement2DReachCheck<T, X, Y>, Added<LinearTargetReached<T, Y>>>,
) {
    for mut check in query.iter_mut() {
        check.reached.1 = true;
    }
}
