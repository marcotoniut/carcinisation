use bevy::prelude::*;
use seldom_pixel::{asset::*, prelude::*};

use crate::{
    globals::{SCREEN_RESOLUTION, TYPEFACE_CHARACTERS, TYPEFACE_INVERTED_PATH},
    Layer,
};

use super::components::StageClearedText;

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
    info!("skybox: {}", skybox_path);

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

// TODO should be in ui
pub fn make_stage_cleared_bundle(
    assets_sprite: &mut PxAssets<PxSprite>,
    typefaces: &mut PxAssets<PxTypeface>,
    filters: &mut PxAssets<PxFilter>,
) -> (PxTextBundle<Layer>, StageClearedText, Name) {
    let typeface = typefaces.load(TYPEFACE_INVERTED_PATH, TYPEFACE_CHARACTERS, [(' ', 4)]);

    (
        PxTextBundle::<Layer> {
            alignment: PxAnchor::Center,
            canvas: PxCanvas::Camera,
            layer: Layer::UI,
            rect: IRect::new(
                IVec2::new(0, 0),
                IVec2::new(SCREEN_RESOLUTION.x as i32, SCREEN_RESOLUTION.y as i32),
            )
            .into(),
            text: "STAGE CLEARED!".into(),
            typeface: typeface.clone(),
            ..default()
        },
        StageClearedText {},
        Name::new("StageClearedText"),
    )
}
