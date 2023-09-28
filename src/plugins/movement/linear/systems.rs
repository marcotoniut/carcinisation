use bevy::prelude::*;

use crate::{core::time::DeltaTime, plugins::movement::structs::Magnitude};

use super::components::*;

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
            &mut P,
            &LinearSpeed<T, P>,
            &LinearTargetPosition<T, P>,
        ),
        Without<LinearTargetReached<T, P>>,
    >,
) {
    for (entity, mut position, speed, target) in &mut query.iter_mut() {
        let x = position.get();
        if (speed.value < 0. && x <= target.value) || (speed.value > 0. && x >= target.value) {
            position.set(target.value);
            commands
                .entity(entity)
                .remove::<LinearTargetPosition<T, P>>()
                .insert(LinearTargetReached::<T, P>::new());
        }
    }
}
