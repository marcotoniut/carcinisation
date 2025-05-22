use super::data::SkyboxData;
use crate::layer::Layer;
use bevy::prelude::*;
use seldom_pixel::{asset::*, prelude::*};

#[derive(Bundle)]
pub struct BackgroundBundle {
    pub name: Name,
    pub position: PxSubPosition,
    pub sprite: PxSprite,
    anchor: PxAnchor,
    layer: Layer,
}

impl BackgroundBundle {
    pub fn new(sprite: PxSprite) -> Self {
        Self {
            name: Name::new("Background"),
            position: Vec2::ZERO.into(),
            sprite,
            anchor: PxAnchor::BottomLeft,
            layer: Layer::Background,
        }
    }
}

#[derive(Bundle)]
pub struct SkyboxBundle {
    pub animation: PxAnimation,
    pub name: Name,
    pub position: PxSubPosition,
    pub sprite: PxSprite,
    pub anchor: PxAnchor,
    pub canvas: PxCanvas,
    pub layer: Layer,
}

impl SkyboxBundle {
    pub fn new(asset_server: &Res<AssetServer>, skybox_data: SkyboxData) -> Self {
        let sprite = PxSprite(asset_server.load(skybox_data.path));
        // TODO animate skybox_data.frames

        Self {
            animation: PxAnimation {
                duration: PxAnimationDuration::millis_per_animation(2000),
                on_finish: PxAnimationFinishBehavior::Loop,
                ..default()
            },
            name: Name::new("Skybox"),
            position: Vec2::ZERO.into(),
            sprite,
            anchor: PxAnchor::BottomLeft,
            canvas: PxCanvas::Camera,
            layer: Layer::Skybox,
        }
    }
}
