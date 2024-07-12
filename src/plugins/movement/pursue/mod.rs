pub mod components;
mod systems;

use self::systems::*;
use super::structs::MovementVec2Position;
use crate::core::time::DeltaTime;
use bevy::prelude::*;
use std::marker::PhantomData;

pub struct PursueMovementPlugin<
    T: DeltaTime + 'static + Resource,
    P: MovementVec2Position + 'static + Component,
> {
    _phantom_t: PhantomData<T>,
    _phantom_p: PhantomData<P>,
}

impl<T: DeltaTime + 'static + Resource, P: MovementVec2Position + Component> Default
    for PursueMovementPlugin<T, P>
{
    fn default() -> Self {
        Self {
            _phantom_t: PhantomData,
            _phantom_p: PhantomData,
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
