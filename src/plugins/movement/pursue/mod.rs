pub mod components;
pub mod systems;

use std::marker::PhantomData;

use bevy::prelude::*;

use crate::core::time::DeltaTime;

use self::systems::*;

use super::structs::MovementVec2Position;

pub struct PursueMovementPlugin<
    T: DeltaTime + 'static + Resource,
    P: MovementVec2Position + 'static + Component,
> {
    _marker: PhantomData<T>,
    _marker_position: PhantomData<P>,
}

impl<T: DeltaTime + 'static + Resource, P: MovementVec2Position + Component> Default
    for PursueMovementPlugin<T, P>
{
    fn default() -> Self {
        Self {
            _marker: PhantomData,
            _marker_position: PhantomData,
        }
    }
}

impl<T: DeltaTime + 'static + Resource, P: MovementVec2Position + Component> Plugin
    for PursueMovementPlugin<T, P>
{
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                update::<T, P>,
                check_x_reached::<T, P>,
                check_y_reached::<T, P>,
                check_reached::<T, P>,
                on_position_added::<T, P>,
            ),
        );
    }
}
