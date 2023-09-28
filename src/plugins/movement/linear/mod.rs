pub mod components;
pub mod systems;

use self::systems::*;
use super::structs::Magnitude;
use crate::core::time::DeltaTime;
use bevy::prelude::*;
use std::marker::PhantomData;

pub struct LinearMovementPlugin<
    T: DeltaTime + 'static + Resource,
    P: Magnitude + 'static + Component,
> {
    _marker: PhantomData<T>,
    _marker_position: PhantomData<P>,
}

impl<T: DeltaTime + 'static + Resource, P: Magnitude + Component> Default
    for LinearMovementPlugin<T, P>
{
    fn default() -> Self {
        Self {
            _marker: PhantomData,
            _marker_position: PhantomData,
        }
    }
}

impl<T: DeltaTime + 'static + Resource, P: Magnitude + Component> Plugin
    for LinearMovementPlugin<T, P>
{
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                update::<T, P>,
                update_speed::<T, P>,
                check_reached::<T, P>,
                on_position_added::<T, P>,
                on_reached::<T, P>,
            ),
        );
    }
}
