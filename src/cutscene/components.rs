use crate::{
    cutscene::data::CutsceneLayer, globals::SCREEN_RESOLUTION, pixel::components::PxRectangle,
    Layer,
};
use bevy::prelude::*;
use seldom_pixel::{
    asset::*,
    filter::*,
    prelude::{PxAnchor, PxCanvas, PxSubPosition},
};

#[derive(Component)]
pub struct Cinematic;

#[derive(Component)]
pub struct CutsceneEntity;

#[derive(Component)]
pub struct CutsceneGraphic;

#[derive(Component)]
pub struct LetterboxBottom;

#[derive(Component)]
pub struct LetterboxTop;

pub const LETTERBOX_UPDATE_TIME: f32 = 0.015;

pub const LETTERBOX_HEIGHT: u32 = 30;

pub fn build_letterbox_top(commands: &mut Commands, filter: &Handle<PxAsset<PxFilterData>>) {
    commands.spawn((
        Name::new("LetterboxTop".to_string()),
        LetterboxTop,
        PxSubPosition(Vec2::new(
            0.,
            SCREEN_RESOLUTION.y as f32 - LETTERBOX_HEIGHT as f32,
        )),
        PxRectangle {
            canvas: PxCanvas::Camera,
            width: SCREEN_RESOLUTION.x,
            height: SCREEN_RESOLUTION.y,
            filter: filter.clone(),
            layer: Layer::CutsceneLayer(CutsceneLayer::Letterbox),
            anchor: PxAnchor::BottomLeft,
        },
    ));
}

pub fn build_letterbox_bottom(commands: &mut Commands, filter: &Handle<PxAsset<PxFilterData>>) {
    commands.spawn((
        Name::new("LetterboxBottom".to_string()),
        LetterboxBottom,
        PxSubPosition(Vec2::new(0., 0. + LETTERBOX_HEIGHT as f32)),
        PxRectangle {
            canvas: PxCanvas::Camera,
            width: SCREEN_RESOLUTION.x,
            height: SCREEN_RESOLUTION.y,
            filter: filter.clone(),
            layer: Layer::CutsceneLayer(CutsceneLayer::Letterbox),
            anchor: PxAnchor::TopLeft,
        },
    ));
}
