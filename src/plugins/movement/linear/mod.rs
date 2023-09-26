pub mod components;
pub mod systems;

use bevy::prelude::*;

use self::{components::DeltaTime, systems::*};

pub struct LinearMovementPlugin<T: DeltaTime + 'static + Resource> {
    _marker: std::marker::PhantomData<T>,
}

impl<T: DeltaTime + 'static + Resource> Default for LinearMovementPlugin<T> {
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T: DeltaTime + 'static + Resource> Plugin for LinearMovementPlugin<T> {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                move_linear::<T>,
                check_linear_x_reached::<T>,
                check_linear_y_reached::<T>,
                check_linear_reached::<T>,
            ),
        );
    }
}
