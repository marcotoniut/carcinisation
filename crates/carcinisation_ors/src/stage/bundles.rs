//! Convenience Bevy bundles for background and skybox entities in the stage.

use super::data::SkyboxData;
use crate::assets::CxAssets;
use bevy::prelude::*;
use carapace::prelude::{
    CxAnchor, CxAnimationBundle, CxAnimationDirection, CxAnimationDuration,
    CxAnimationFinishBehavior, CxFrameTransition, CxRenderSpace, CxSprite, CxSpriteBundle,
    WorldPos,
};
use carapace::sprite::CxSpriteAsset;
use carcinisation_base::layer::Layer;
use carcinisation_base::layer::OrsLayer;
use carcinisation_base::layer::SharedLayer;

#[derive(Bundle)]
/// Pixel background sprite anchored to the bottom-left of the world.
pub struct BackgroundBundle {
    pub name: Name,
    pub position: WorldPos,
    pub sprite: CxSpriteBundle<Layer>,
}

impl BackgroundBundle {
    /// Creates a background sprite bundle using the provided pixel sprite handle.
    #[must_use]
    pub fn new(sprite: Handle<CxSpriteAsset>) -> Self {
        Self {
            name: Name::new("Background"),
            position: Vec2::ZERO.into(),
            sprite: CxSpriteBundle::<Layer> {
                sprite: sprite.into(),
                anchor: CxAnchor::BottomLeft,
                layer: Layer::Ors(OrsLayer::Background),
                ..default()
            },
        }
    }
}

#[derive(Bundle)]
/// Animated skybox that plays on the camera canvas.
pub struct SkyboxBundle {
    pub animation: CxAnimationBundle,
    pub name: Name,
    pub position: WorldPos,
    pub sprite: CxSpriteBundle<Layer>,
}

impl SkyboxBundle {
    /// Builds the skybox bundle from serialized skybox data.
    pub fn new(assets_sprite: &mut CxAssets<CxSprite>, skybox_data: SkyboxData) -> Self {
        let sprite = assets_sprite.load_animated(skybox_data.path, skybox_data.frames);

        Self {
            animation: CxAnimationBundle::from_parts(
                CxAnimationDirection::default(),
                CxAnimationDuration::millis_per_animation(2000),
                CxAnimationFinishBehavior::Loop,
                CxFrameTransition::default(),
            ),
            name: Name::new("Skybox"),
            position: Vec2::ZERO.into(),
            sprite: CxSpriteBundle::<Layer> {
                sprite: sprite.into(),
                anchor: CxAnchor::BottomLeft,
                canvas: CxRenderSpace::Camera,
                layer: Layer::Shared(SharedLayer::Skybox),
                ..default()
            },
        }
    }
}
