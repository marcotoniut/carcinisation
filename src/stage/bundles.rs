//! Convenience Bevy bundles for background and skybox entities in the stage.

use super::data::SkyboxData;
use crate::{
    layer::Layer,
    pixel::{PxAnimationBundle, PxAssets, PxSpriteBundle},
};
use bevy::prelude::*;
use seldom_pixel::prelude::{
    PxAnchor, PxAnimationDirection, PxAnimationDuration, PxAnimationFinishBehavior,
    PxAnimationFrameTransition, PxCanvas, PxSprite, PxSubPosition,
};
use seldom_pixel::sprite::PxSpriteAsset;

#[derive(Bundle)]
/// Pixel background sprite anchored to the bottom-left of the world.
pub struct BackgroundBundle {
    pub name: Name,
    pub position: PxSubPosition,
    pub sprite: PxSpriteBundle<Layer>,
}

impl BackgroundBundle {
    /// Creates a background sprite bundle using the provided pixel sprite handle.
    pub fn new(sprite: Handle<PxSpriteAsset>) -> Self {
        Self {
            name: Name::new("Background"),
            position: Vec2::ZERO.into(),
            sprite: PxSpriteBundle::<Layer> {
                sprite: sprite.into(),
                anchor: PxAnchor::BottomLeft,
                layer: Layer::Background,
                ..default()
            },
        }
    }
}

#[derive(Bundle)]
/// Animated skybox that plays on the camera canvas.
pub struct SkyboxBundle {
    pub animation: PxAnimationBundle,
    pub name: Name,
    pub position: PxSubPosition,
    pub sprite: PxSpriteBundle<Layer>,
}

impl SkyboxBundle {
    /// Builds the skybox bundle from serialized skybox data.
    pub fn new(assets_sprite: &mut PxAssets<PxSprite>, skybox_data: SkyboxData) -> Self {
        let sprite = assets_sprite.load_animated(skybox_data.path, skybox_data.frames);

        Self {
            animation: PxAnimationBundle::from_parts(
                PxAnimationDirection::default(),
                PxAnimationDuration::millis_per_animation(2000),
                PxAnimationFinishBehavior::Loop,
                PxAnimationFrameTransition::default(),
            ),
            name: Name::new("Skybox"),
            position: Vec2::ZERO.into(),
            sprite: PxSpriteBundle::<Layer> {
                sprite: sprite.into(),
                anchor: PxAnchor::BottomLeft,
                canvas: PxCanvas::Camera,
                layer: Layer::Skybox,
                ..default()
            },
        }
    }
}
