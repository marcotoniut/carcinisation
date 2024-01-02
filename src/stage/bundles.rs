use super::data::SkyboxData;
use crate::Layer;
use bevy::prelude::*;
use seldom_pixel::{asset::*, prelude::*};

#[derive(Bundle)]
pub struct BackgroundBundle {
    pub name: Name,
    pub position: PxSubPosition,
    pub sprite: PxSpriteBundle<Layer>,
}

impl BackgroundBundle {
    pub fn new(sprite: Handle<PxSprite>) -> Self {
        Self {
            name: Name::new("Background"),
            position: Vec2::ZERO.into(),
            sprite: PxSpriteBundle::<Layer> {
                sprite,
                anchor: PxAnchor::BottomLeft,
                layer: Layer::Background,
                ..Default::default()
            },
        }
    }
}

#[derive(Bundle)]
pub struct SkyboxBundle {
    pub animation: PxAnimationBundle,
    pub name: Name,
    pub position: PxSubPosition,
    pub sprite: PxSpriteBundle<Layer>,
}

impl SkyboxBundle {
    pub fn new(assets_sprite: &mut PxAssets<PxSprite>, skybox_data: SkyboxData) -> Self {
        let sprite = assets_sprite.load_animated(skybox_data.path, skybox_data.frames);

        Self {
            animation: PxAnimationBundle {
                duration: PxAnimationDuration::millis_per_animation(2000),
                on_finish: PxAnimationFinishBehavior::Loop,
                ..Default::default()
            },
            name: Name::new("Skybox"),
            position: Vec2::ZERO.into(),
            sprite: PxSpriteBundle::<Layer> {
                sprite,
                anchor: PxAnchor::BottomLeft,
                canvas: PxCanvas::Camera,
                layer: Layer::Skybox,
                ..Default::default()
            },
        }
    }
}
