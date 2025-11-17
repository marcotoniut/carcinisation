pub mod components;
mod systems;

use self::systems::*;
use super::structs::MovementVec2Position;
use bevy::{ecs::component::Mutable, prelude::*};
use std::marker::PhantomData;

pub struct PursueMovementPlugin<
    D: Default + Send + Sync + 'static,
    P: MovementVec2Position + 'static + Component<Mutability = Mutable>,
> {
    _phantom_d: PhantomData<D>,
    _phantom_p: PhantomData<P>,
}

impl<D, P> Default for PursueMovementPlugin<D, P>
where
    D: Default + Send + Sync + 'static,
    P: MovementVec2Position + Component<Mutability = Mutable>,
{
    fn default() -> Self {
        Self {
            _phantom_d: PhantomData,
            _phantom_p: PhantomData,
        }
    }
}

impl<D, P> Plugin for PursueMovementPlugin<D, P>
where
    D: Default + Send + Sync + 'static,
    P: MovementVec2Position + Component<Mutability = Mutable>,
{
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (
                update::<D, P>,
                check_x_reached::<D, P>,
                check_y_reached::<D, P>,
                check_reached::<D, P>,
                on_position_added::<D, P>,
            ),
        );
    }
}
