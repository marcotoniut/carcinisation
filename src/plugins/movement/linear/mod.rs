pub mod components;
pub mod systems;

use std::marker::PhantomData;

use bevy::prelude::*;

use self::{
    components::{DeltaTime, LinearPosition},
    systems::*,
};

pub struct LinearMovementPlugin<
    T: DeltaTime + 'static + Resource,
    P: LinearPosition + 'static + Component,
> {
    _marker: PhantomData<T>,
    _marker_position: PhantomData<P>,
}

impl<T: DeltaTime + 'static + Resource, P: LinearPosition + 'static + Component> Default
    for LinearMovementPlugin<T, P>
{
    fn default() -> Self {
        Self {
            _marker: PhantomData,
            _marker_position: PhantomData,
        }
    }
}

impl<T: DeltaTime + 'static + Resource, P: LinearPosition + 'static + Component> Plugin
    for LinearMovementPlugin<T, P>
{
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                move_linear::<T, P>,
                check_linear_x_reached::<T, P>,
                check_linear_y_reached::<T, P>,
                check_linear_reached::<T>,
            ),
        );
    }
}
