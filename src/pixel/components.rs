use bevy::prelude::*;
use seldom_pixel::{asset::PxAsset, filter::PxFilterData, prelude::*};

#[derive(Component, Default)]
pub struct PxRectangle<L: PxLayer> {
    pub canvas: PxCanvas,
    pub filter: Handle<PxAsset<PxFilterData>>,
    pub layer: L,
    pub height: u32,
    pub width: u32,
}

#[derive(Component)]
pub struct PxRectangleRow(pub u32);
