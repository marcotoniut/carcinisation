pub mod components;
mod systems;

use self::systems::*;
use bevy::prelude::*;
use seldom_pixel::prelude::PxLayer;
use std::marker::PhantomData;

pub struct PixelPlugin<L: PxLayer> {
    _phantom_l: PhantomData<L>,
}

impl<L: PxLayer> Default for PixelPlugin<L> {
    fn default() -> Self {
        Self {
            _phantom_l: PhantomData,
        }
    }
}

impl<L: PxLayer> Plugin for PixelPlugin<L> {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, construct_rectangle::<L>)
            .add_systems(
                Update,
                (update_rectangle_color::<L>, update_rectangle_position::<L>).chain(),
            );
    }
}
