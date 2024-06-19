use super::components::*;
use super::events::LetterboxMoveEvent;
use super::resources::LetterboxTime;
use crate::globals::mark_for_despawn_by_query;
use crate::globals::GBColor;
use crate::plugins::movement::linear::components::LinearMovementBundle;
// use crate::plugins::movement::linear::components::LinearPositionRemovalBundle;
use crate::plugins::movement::linear::components::TargetingPositionY;
use crate::{
    cutscene::data::CutsceneLayer, globals::SCREEN_RESOLUTION, layer::Layer,
    pixel::components::PxRectangle,
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
    mark_for_despawn_by_query(&mut commands, &entity_query);
}

pub fn on_move(
    mut commands: Commands,
    mut event_reader: EventReader<LetterboxMoveEvent>,
    top_query: Query<(Entity, &PxSubPosition), With<LetterboxTop>>,
    bottom_query: Query<(Entity, &PxSubPosition), With<LetterboxBottom>>,
) {
    for e in event_reader.read() {
        for (entity, position) in top_query.iter() {
            insert_linear_movement(
                &mut commands,
                (entity, position),
                SCREEN_RESOLUTION.y as f32 - e.target,
                e.speed,
            );
        }

        for (entity, position) in bottom_query.iter() {
            insert_linear_movement(&mut commands, (entity, position), e.target, e.speed);
        }
    }
}

pub fn insert_linear_movement(
    commands: &mut Commands,
    (entity, position): (Entity, &PxSubPosition),
    target: f32,
    speed: f32,
) {
    let speed = speed * (target - position.y).signum();
    commands
        .entity(entity)
        // TODO review why this was removed
        // .remove::<LinearPositionRemovalBundle<LetterboxTime, TargetingPositionY>>()
        .insert(
            LinearMovementBundle::<LetterboxTime, TargetingPositionY>::new(
                position.y, target, speed,
            ),
        );
}
