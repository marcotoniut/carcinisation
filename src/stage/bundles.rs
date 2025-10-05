//! Convenience Bevy bundles for background and skybox entities in the stage.

use super::data::SkyboxData;
use crate::layer::Layer;
use bevy::prelude::*;
use seldom_pixel::{asset::*, prelude::*};

#[derive(Bundle)]
/// Pixel background sprite anchored to the bottom-left of the world.
pub struct BackgroundBundle {
    pub name: Name,
    pub position: PxSubPosition,
    pub sprite: PxSpriteBundle<Layer>,
}

impl BackgroundBundle {
    /// Creates a background sprite bundle using the provided pixel sprite handle.
    pub fn new(sprite: Handle<PxSprite>) -> Self {
        Self {
            name: Name::new("Background"),
            position: Vec2::ZERO.into(),
            sprite: PxSpriteBundle::<Layer> {
                sprite,
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
            animation: PxAnimationBundle {
                duration: PxAnimationDuration::millis_per_animation(2000),
                on_finish: PxAnimationFinishBehavior::Loop,
                ..default()
            },
            name: Name::new("Skybox"),
            position: Vec2::ZERO.into(),
            sprite: PxSpriteBundle::<Layer> {
                sprite,
                anchor: PxAnchor::BottomLeft,
                canvas: PxCanvas::Camera,
                layer: Layer::Skybox,
                ..default()
            },
        }
    }
}
