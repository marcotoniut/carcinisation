pub mod components;
mod systems;

use self::systems::*;
use super::structs::Magnitude;
use bevy::{ecs::component::Mutable, prelude::*};
use std::marker::PhantomData;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct LinearTweenSystems;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct LinearTweenCleanupSystems;

pub struct LinearTweenPlugin<
    D: Default + Send + Sync + 'static,
    P: Magnitude + 'static + Component<Mutability = Mutable>,
> {
    _phantom_d: PhantomData<D>,
    _phantom_p: PhantomData<P>,
}

impl<D, P> Default for LinearTweenPlugin<D, P>
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

impl<D, P> Plugin for LinearTweenPlugin<D, P>
where
    D: Default + Send + Sync + 'static,
    P: Magnitude + Component<Mutability = Mutable>,
{
    fn build(&self, app: &mut App) {
        app.configure_sets(FixedUpdate, LinearTweenSystems);
        app.configure_sets(PostUpdate, LinearTweenCleanupSystems);
        app.add_systems(
            FixedUpdate,
            (
                on_value_added::<D, P>,
                (
                    update::<D, P>,
                    check_value_reached::<D, P>,
                    propagate_child_reached_to_parent::<D, P>,
                )
                    .chain(),
                aggregate_tween_children_to_parent::<D, P>,
            )
                .chain()
                .in_set(LinearTweenSystems),
        );
        // Cleanup runs exclusively to avoid deferred-command races.
        app.add_systems(
            FixedUpdate,
            on_reached_cleanup::<D, P>
                .in_set(LinearTweenCleanupSystems)
                .after(LinearTweenSystems),
        );
    }
}

pub struct LinearTween2DPlugin<
    D: Default + Send + Sync + 'static,
    X: Magnitude + 'static + Component<Mutability = Mutable>,
    Y: Magnitude + 'static + Component<Mutability = Mutable>,
> {
    _phantom_d: PhantomData<D>,
    _phantom_x: PhantomData<X>,
    _phantom_y: PhantomData<Y>,
}

impl<D, X, Y> Default for LinearTween2DPlugin<D, X, Y>
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

impl<D, X, Y> Plugin for LinearTween2DPlugin<D, X, Y>
where
    D: Default + Send + Sync + 'static,
    X: Magnitude + Component<Mutability = Mutable>,
    Y: Magnitude + Component<Mutability = Mutable>,
{
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (check_2d_x_reached::<D, X, Y>, check_2d_y_reached::<D, X, Y>)
                .after(LinearTweenSystems),
        );
    }
}
