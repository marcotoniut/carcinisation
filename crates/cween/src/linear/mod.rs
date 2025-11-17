pub mod components;
mod systems;

use self::systems::*;
use super::structs::Magnitude;
use bevy::{ecs::component::Mutable, prelude::*};
use std::marker::PhantomData;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct LinearMovementSystems;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct LinearMovementCleanupSystems;

pub struct LinearMovementPlugin<
    D: Default + Send + Sync + 'static,
    P: Magnitude + 'static + Component<Mutability = Mutable>,
> {
    _phantom_d: PhantomData<D>,
    _phantom_p: PhantomData<P>,
}

impl<D, P> Default for LinearMovementPlugin<D, P>
where
    D: Default + Send + Sync + 'static,
    P: Magnitude + Component<Mutability = Mutable>,
{
    fn default() -> Self {
        Self {
            _phantom_d: PhantomData,
            _phantom_p: PhantomData,
        }
    }
}

impl<D, P> Plugin for LinearMovementPlugin<D, P>
where
    D: Default + Send + Sync + 'static,
    P: Magnitude + Component<Mutability = Mutable>,
{
    fn build(&self, app: &mut App) {
        app.configure_sets(FixedUpdate, LinearMovementSystems);
        app.configure_sets(PostUpdate, LinearMovementCleanupSystems);
        app.add_systems(
            FixedUpdate,
            (
                on_position_added::<D, P>,
                (
                    update::<D, P>,
                    check_reached::<D, P>,
                    propagate_child_reached_to_parent::<D, P>,
                )
                    .chain(),
                aggregate_movement_children_to_parent::<D, P>,
            )
                .chain()
                .in_set(LinearMovementSystems),
        );
        // Cleanup runs exclusively to avoid deferred-command races.
        app.add_systems(
            FixedUpdate,
            on_reached_cleanup::<D, P>
                .in_set(LinearMovementCleanupSystems)
                .after(LinearMovementSystems),
        );
    }
}

pub struct LinearMovement2DPlugin<
    D: Default + Send + Sync + 'static,
    X: Magnitude + 'static + Component<Mutability = Mutable>,
    Y: Magnitude + 'static + Component<Mutability = Mutable>,
> {
    _phantom_d: PhantomData<D>,
    _phantom_x: PhantomData<X>,
    _phantom_y: PhantomData<Y>,
}

impl<D, X, Y> Default for LinearMovement2DPlugin<D, X, Y>
where
    D: Default + Send + Sync + 'static,
    X: Magnitude + Component<Mutability = Mutable>,
    Y: Magnitude + 'static + Component<Mutability = Mutable>,
{
    fn default() -> Self {
        Self {
            _phantom_d: PhantomData,
            _phantom_x: PhantomData,
            _phantom_y: PhantomData,
        }
    }
}

impl<D, X, Y> Plugin for LinearMovement2DPlugin<D, X, Y>
where
    D: Default + Send + Sync + 'static,
    X: Magnitude + Component<Mutability = Mutable>,
    Y: Magnitude + Component<Mutability = Mutable>,
{
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (check_2d_x_reached::<D, X, Y>, check_2d_y_reached::<D, X, Y>)
                .after(LinearMovementSystems),
        );
    }
}
