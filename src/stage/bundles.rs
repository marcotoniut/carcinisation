use bevy::prelude::*;
use seldom_pixel::{asset::*, prelude::*};

use crate::Layer;

pub fn make_background_bundle(
    assets_sprite: &mut PxAssets<PxSprite>,
    background_path: String,
) -> (PxSpriteBundle<Layer>, PxSubPosition, Name) {
    let sprite = assets_sprite.load(background_path);
    (
        PxSpriteBundle::<Layer> {
            sprite,
            anchor: PxAnchor::BottomLeft,
            layer: Layer::Back,
            ..default()
        },
        PxSubPosition::from(Vec2::new(0.0, 0.0)),
        Name::new("Background"),
    )
}

pub fn make_skybox_bundle(
    assets_sprite: &mut PxAssets<PxSprite>,
    skybox_path: String,
) -> (PxSpriteBundle<Layer>, PxSubPosition, Name) {
    let sprite = assets_sprite.load(skybox_path);
    return (
        PxSpriteBundle::<Layer> {
            sprite,
            anchor: PxAnchor::BottomLeft,
            canvas: PxCanvas::Camera,
            layer: Layer::Skybox,
            ..default()
        },
        PxSubPosition::from(Vec2::new(0.0, 0.0)),
        Name::new("Skybox"),
    );
}
