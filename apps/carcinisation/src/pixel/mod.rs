//! Integrates `seldom_pixel` helpers for rendering and assets.

pub mod assets;
pub mod bundle;

use bevy::prelude::*;
use seldom_pixel::prelude::PxLayer;
use std::marker::PhantomData;

pub use assets::{PxAsset, PxAssets, PxSpriteData};
pub use bundle::{PxAnimationBundle, PxLineBundle, PxRectBundle, PxSpriteBundle, PxTextBundle};

/// Wraps pixel-specific systems for constructing/updating rectangle gizmos.
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
        let _ = app;
    }
}
