use bevy::prelude::*;

use crate::{
    core::time::DeltaTime,
    plugins::movement::structs::{Magnitude, MovementDirection},
};

use super::components::{extra::LinearMovement2DReachCheck, *};

pub fn update<T: DeltaTime + 'static + Resource, P: Magnitude + Component>(
    mut query: Query<(&mut P, &LinearSpeed<T, P>), Without<LinearTargetReached<T, P>>>,
    delta_time: Res<T>,
) {
    for (mut position, speed) in &mut query.iter_mut() {
        position.add(speed.value * delta_time.delta_seconds());
    }
}

pub fn update_speed<T: DeltaTime + 'static + Resource, P: Magnitude + Component>(
    mut query: Query<(&mut LinearSpeed<T, P>, &LinearAcceleration<T, P>)>,
    delta_time: Res<T>,
) {
    for (mut speed, acceleration) in &mut query.iter_mut() {
        speed.add(acceleration.value * delta_time.delta_seconds());
    }
}

/**
 * What to do if there's already a bundle? Should I simply clean it up on added?
 */
pub fn on_position_added<T: DeltaTime + 'static + Resource, P: Magnitude + Component>(
    mut commands: Commands,
    mut query: Query<(Entity, Added<LinearTargetPosition<T, P>>)>,
) {
    for (entity, added) in &mut query.iter_mut() {
        if added {
            commands
                .entity(entity)
                .remove::<LinearTargetReached<T, P>>();
        }
    }
}

pub fn check_reached<T: DeltaTime + 'static + Resource, P: Magnitude + Component>(
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &P,
            &LinearDirection<T, P>,
            &LinearTargetPosition<T, P>,
        ),
        Without<LinearTargetReached<T, P>>,
    >,
) {
    for (entity, position, direction, target) in &mut query.iter_mut() {
        let x = position.get();
        if (direction.value == MovementDirection::Negative && x <= target.value)
            || (direction.value == MovementDirection::Positive && x >= target.value)
        {
            commands
                .entity(entity)
                .insert(LinearTargetReached::<T, P>::new());
        }
    }
}

pub fn on_reached<T: DeltaTime + 'static + Resource, P: Magnitude + Component>(
    mut commands: Commands,
    mut query: Query<
        (Entity, &mut P, &LinearTargetPosition<T, P>),
        Added<LinearTargetReached<T, P>>,
    >,
) {
    for (entity, mut position, target) in &mut query.iter_mut() {
        position.set(target.value);
        // TODO remove bundle
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
