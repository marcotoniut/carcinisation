use super::components::*;
use crate::globals::mark_for_despawn_by_component_query;
use crate::globals::GBColor;
use crate::{
    cutscene::data::CutsceneLayer, globals::SCREEN_RESOLUTION, pixel::components::PxRectangle,
    Layer,
};
use bevy::prelude::*;
use seldom_pixel::prelude::PxSubPosition;
use seldom_pixel::prelude::*;

pub fn on_startup(mut commands: Commands) {
    let color = GBColor::Black;

    commands.spawn((
        Name::new("LetterboxTop".to_string()),
        LetterboxEntity,
        LetterboxTop,
        PxSubPosition(Vec2::new(0., SCREEN_RESOLUTION.y as f32)),
        PxRectangle {
            canvas: PxCanvas::Camera,
            color,
            width: SCREEN_RESOLUTION.x,
            height: SCREEN_RESOLUTION.y,
            layer: Layer::CutsceneLayer(CutsceneLayer::Letterbox),
            anchor: PxAnchor::BottomLeft,
        },
    ));

    commands.spawn((
        Name::new("LetterboxBottom".to_string()),
        LetterboxEntity,
        LetterboxBottom,
        PxSubPosition(Vec2::ZERO),
        PxRectangle {
            canvas: PxCanvas::Camera,
            color,
            width: SCREEN_RESOLUTION.x,
            height: SCREEN_RESOLUTION.y,
            layer: Layer::CutsceneLayer(CutsceneLayer::Letterbox),
            anchor: PxAnchor::TopLeft,
        },
    ));
}

pub fn on_shutdown(mut commands: Commands, entity_query: Query<Entity, With<LetterboxEntity>>) {
    mark_for_despawn_by_component_query(&mut commands, &entity_query);
}
