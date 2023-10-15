use super::data::SkyboxData;
use crate::Layer;
use bevy::prelude::*;
use seldom_pixel::{asset::*, prelude::*};

pub fn make_background_bundle(
    assets_sprite: &mut PxAssets<PxSprite>,
    background_path: String,
) -> (PxSpriteBundle<Layer>, PxSubPosition, Name) {
    info!("background: {}", background_path);

    let sprite = assets_sprite.load(background_path);
    (
        PxSpriteBundle::<Layer> {
            sprite,
            anchor: PxAnchor::BottomLeft,
            layer: Layer::Background,
            ..Default::default()
        },
        PxSubPosition::from(Vec2::new(0.0, 0.0)),
        Name::new("Background"),
    )
}

pub fn make_skybox_bundle(
    assets_sprite: &mut PxAssets<PxSprite>,
    skybox_data: SkyboxData,
) -> (
    PxSpriteBundle<Layer>,
    PxAnimationBundle,
    PxSubPosition,
    Name,
) {
    info!("skybox: {}", skybox_data.path);

    let sprite = assets_sprite.load_animated(skybox_data.path, skybox_data.frames);
    (
        PxSpriteBundle::<Layer> {
            sprite,
            anchor: PxAnchor::BottomLeft,
            canvas: PxCanvas::Camera,
            layer: Layer::Skybox,
            ..Default::default()
        },
        PxAnimationBundle {
            // TODO variable time
            duration: PxAnimationDuration::millis_per_animation(2000),
            on_finish: PxAnimationFinishBehavior::Loop,
            ..Default::default()
        },
        PxSubPosition::from(Vec2::new(0.0, 0.0)),
        Name::new("Skybox"),
    )
}
