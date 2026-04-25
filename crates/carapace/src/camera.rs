use bevy_derive::{Deref, DerefMut};
#[cfg(feature = "headed")]
use bevy_render::{
    extract_component::ExtractComponent,
    extract_resource::{ExtractResource, ExtractResourcePlugin},
};

use crate::prelude::*;

pub(crate) fn plug_core(app: &mut App) {
    app.init_resource::<CxCamera>();
}

pub(crate) fn plug(app: &mut App) {
    plug_core(app);
    #[cfg(feature = "headed")]
    app.add_plugins(ExtractResourcePlugin::<CxCamera>::default());
}

/// The camera's integer position in world space.
///
/// Offsets all [`CxRenderSpace::World`] entities when rendering.  Screen-space
/// entities ([`CxRenderSpace::Camera`]) are unaffected.
#[cfg_attr(feature = "headed", derive(ExtractResource))]
#[derive(Resource, Deref, DerefMut, Clone, Copy, Default, PartialEq, Eq, Debug, Reflect)]
pub struct CxCamera(pub IVec2);

/// Coordinate space selector for a drawable entity.
///
/// Controls how an entity's [`CxPosition`](crate::position::CxPosition) is
/// interpreted during rendering:
///
/// - [`World`](Self::World) — position is in **world space**, offset by the
///   camera.  Terrain, enemies, pickups, and other gameplay entities use this.
/// - [`Camera`](Self::Camera) — position is in **screen space**, fixed
///   relative to the viewport.  HUD, menus, and overlays use this.
#[cfg_attr(feature = "headed", derive(ExtractComponent))]
#[derive(Component, Clone, Copy, Default, PartialEq, Eq, Reflect, Debug)]
pub enum CxRenderSpace {
    /// Drawn relative to the world origin, offset by the camera.
    #[default]
    World,
    /// Drawn at a fixed screen position, unaffected by camera movement.
    Camera,
}
