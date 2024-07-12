pub mod components;
mod systems;

use self::systems::*;
use super::structs::Magnitude;
use crate::core::time::DeltaTime;
use bevy::prelude::*;
use std::marker::PhantomData;

pub struct LinearMovementPlugin<
    T: DeltaTime + 'static + Resource,
    P: Magnitude + 'static + Component,
> {
    _phantom_t: PhantomData<T>,
    _phantom_p: PhantomData<P>,
}

impl<T: DeltaTime + 'static + Resource, P: Magnitude + Component> Default
    for LinearMovementPlugin<T, P>
{
    fn default() -> Self {
        Self {
            _phantom_t: PhantomData,
            _phantom_p: PhantomData,
        }
    }
}

impl<T: DeltaTime + 'static + Resource, P: Magnitude + Component> Plugin
    for LinearMovementPlugin<T, P>
{
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, on_position_added::<T, P>)
            .add_systems(
                Update,
                ((on_reached::<T, P>, update::<T, P>, check_reached::<T, P>).chain(),),
            );
    }
}

pub struct LinearMovement2DPlugin<
    T: DeltaTime + 'static + Resource,
    X: Magnitude + 'static + Component,
    Y: Magnitude + 'static + Component,
> {
    _phantom_t: PhantomData<T>,
    _phantom_x: PhantomData<X>,
    _phantom_y: PhantomData<Y>,
}

impl<
        T: DeltaTime + 'static + Resource,
        X: Magnitude + Component,
        Y: Magnitude + 'static + Component,
    > Default for LinearMovement2DPlugin<T, X, Y>
{
    fn default() -> Self {
        Self {
            _phantom_t: PhantomData,
            _phantom_x: PhantomData,
            _phantom_y: PhantomData,
        }
    }
}

impl<T: DeltaTime + 'static + Resource, X: Magnitude + Component, Y: Magnitude + Component> Plugin
    for LinearMovement2DPlugin<T, X, Y>
{
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (check_2d_x_reached::<T, X, Y>, check_2d_y_reached::<T, X, Y>),
        );
    }
}
