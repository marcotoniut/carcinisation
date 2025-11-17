#![allow(clippy::type_complexity)]

use bevy::{
    ecs::{component::Mutable, hierarchy::ChildOf},
    prelude::*,
};

use crate::structs::{Magnitude, TweenDirection};

use super::components::{extra::LinearTween2DReachCheck, *};

pub fn update<D, P>(
    mut query: Query<
        (
            &mut P,
            &mut LinearSpeed<D, P>,
            Option<&LinearAcceleration<D, P>>,
        ),
        Without<LinearValueReached<D, P>>,
    >,
    delta_time: Res<Time<D>>,
) where
    D: Default + Send + Sync + 'static,
    P: Magnitude + Component<Mutability = Mutable>,
{
    for (mut value, mut speed, acceleration_o) in query.iter_mut() {
        if let Some(acceleration) = acceleration_o {
            speed.add(acceleration.value * delta_time.delta().as_secs_f32());
        }
        value.add(speed.value * delta_time.delta().as_secs_f32());
    }
}

/**
 * What to do if there's already a bundle? Should I simply clean it up on added?
 */
pub fn on_value_added<D, P>(
    mut commands: Commands,
    query: Query<(Entity, Option<&ChildOf>), Added<LinearTargetValue<D, P>>>,
) where
    D: Default + Send + Sync + 'static,
    P: Magnitude + Component<Mutability = Mutable>,
{
    for (entity, parent_o) in query.iter() {
        let mut entity_commands = commands.entity(entity);
        entity_commands.remove::<LinearValueReached<D, P>>();

        // If this is a tween child, also clear any stale reach flag on the parent so
        // downstream systems see a fresh Added<LinearValueReached> when the child finishes.
        if let Some(parent) = parent_o {
            commands
                .entity(parent.0)
                .remove::<LinearValueReached<D, P>>();
        }
    }
}

pub fn check_value_reached<D, P>(
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &mut P,
            &LinearTweenDirection<D, P>,
            &LinearTargetValue<D, P>,
        ),
        Without<LinearValueReached<D, P>>,
    >,
) where
    D: Default + Send + Sync + 'static,
    P: Magnitude + Component<Mutability = Mutable>,
{
    for (entity, mut value, direction, target) in query.iter_mut() {
        let current_value = value.get();
        if (direction.value == TweenDirection::Negative && current_value <= target.value)
            || (direction.value == TweenDirection::Positive && current_value >= target.value)
        {
            value.set(target.value);
            commands
                .entity(entity)
                .insert(LinearValueReached::<D, P>::new());
        }
    }
}

/// @system Propagates a tween child's reach to its parent for compatibility with parent-centric
/// systems that listen for Added<LinearValueReached>.
pub fn propagate_child_reached_to_parent<D, P>(
    mut commands: Commands,
    query: Query<&ChildOf, (With<TweenChild>, Added<LinearValueReached<D, P>>)>,
) where
    D: Default + Send + Sync + 'static,
    P: Magnitude + Component<Mutability = Mutable>,
{
    for child_of in query.iter() {
        commands
            .entity(child_of.0)
            .insert(LinearValueReached::<D, P>::new());
    }
}

/// Cleanup tween components once a target is reached.
/// For tween children, clamps value to target and removes tween components,
/// then despawns the child entity.
/// For non-tween-child entities (legacy), removes components without despawning.
/// Uses an exclusive system to avoid deferred-command races with despawns.
pub fn on_reached_cleanup<D, P>(world: &mut World)
where
    D: Default + Send + Sync + 'static,
    P: Magnitude + Component<Mutability = Mutable>,
{
    use crate::linear::components::TweenChild;
    use carcinisation_core::components::DespawnMark;

    let mut query =
        world.query_filtered::<(Entity, Option<&TweenChild>), Added<LinearValueReached<D, P>>>();
    let entities: Vec<(Entity, bool)> =
        query.iter(world).map(|(e, mc)| (e, mc.is_some())).collect();

    for (entity, is_tween_child) in entities {
        if let Ok(mut ew) = world.get_entity_mut(entity) {
            let has_speed = ew.contains::<LinearSpeed<D, P>>();

            // If this is a propagated reach on a parent that has no tween components, leave it
            // in place so downstream parent-centric systems can see the Added<LinearValueReached>.
            if !is_tween_child && !has_speed {
                continue;
            }

            ew.remove::<LinearAcceleration<D, P>>();
            ew.remove::<LinearSpeed<D, P>>();
            ew.remove::<LinearTweenDirection<D, P>>();
            ew.remove::<LinearTargetValue<D, P>>();
            ew.remove::<LinearValueReached<D, P>>();
            ew.remove::<LinearValueRemovalBundle<D, P>>();

            if is_tween_child {
                ew.insert(DespawnMark);
            }
        }
    }
}

/// @system Updates 2D reach check when X-axis tween children reach their target.
/// Looks for tween children that reached their X target and updates the parent's reach check.
pub fn check_2d_x_reached<D, X, Y>(
    mut parent_query: Query<&mut LinearTween2DReachCheck<D, X, Y>>,
    child_query: Query<&ChildOf, (With<TweenChild>, Added<LinearValueReached<D, X>>)>,
) where
    D: Default + Send + Sync + 'static,
    X: Magnitude + Component,
    Y: Magnitude + Component,
{
    for child_of in child_query.iter() {
        if let Ok(mut check) = parent_query.get_mut(child_of.0) {
            check.reached.0 = true;
        }
    }
}

/// @system Updates 2D reach check when Y-axis tween children reach their target.
/// Looks for tween children that reached their Y target and updates the parent's reach check.
pub fn check_2d_y_reached<D, X, Y>(
    mut parent_query: Query<&mut LinearTween2DReachCheck<D, X, Y>>,
    child_query: Query<&ChildOf, (With<TweenChild>, Added<LinearValueReached<D, Y>>)>,
) where
    D: Default + Send + Sync + 'static,
    X: Magnitude + Component,
    Y: Magnitude + Component,
{
    for child_of in child_query.iter() {
        if let Ok(mut check) = parent_query.get_mut(child_of.0) {
            check.reached.1 = true;
        }
    }
}

/// @system Aggregates tween child velocities and integrates them into the parent's value.
/// This system runs after tween children have updated their values and computes
/// the net displacement from all children affecting a given axis.
pub fn aggregate_tween_children_to_parent<D, P>(
    mut parent_query: Query<(Entity, &mut P), Without<TweenChild>>,
    children_query: Query<(&ChildOf, &LinearSpeed<D, P>), With<TweenChild>>,
    delta_time: Res<Time<D>>,
) where
    D: Default + Send + Sync + 'static,
    P: Magnitude + Component<Mutability = Mutable>,
{
    use std::collections::HashMap;

    // Group children by parent and accumulate their velocities
    let mut parent_velocities: HashMap<Entity, f32> = HashMap::new();

    for (child_of, speed) in children_query.iter() {
        *parent_velocities.entry(child_of.0).or_insert(0.0) += speed.value;
    }

    // Apply accumulated velocity to each parent
    let dt = delta_time.delta().as_secs_f32();
    for (parent_entity, mut parent_value) in parent_query.iter_mut() {
        if let Some(&velocity) = parent_velocities.get(&parent_entity) {
            parent_value.add(velocity * dt);
        }
    }
}
