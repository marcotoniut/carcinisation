//! Systems managing the lifecycle and movement of letterbox bars.

use super::components::*;
use super::messages::LetterboxMoveEvent;
use super::resources::LetterboxTimeDomain;
use crate::components::GBColor;
use crate::debug::plugin::debug_print_shutdown;
use crate::debug::plugin::debug_print_startup;
use crate::globals::mark_for_despawn_by_query;
use crate::globals::SCREEN_RESOLUTION_F32;
use crate::{
    cutscene::data::CutsceneLayer,
    globals::SCREEN_RESOLUTION,
    layer::Layer,
    pixel::{PxAssets, PxRectBundle},
};
use bevy::prelude::*;
use cween::linear::components::{LinearTweenBundle, TargetingValueY};
use seldom_pixel::prelude::*;

const DEBUG_MODULE: &str = "Letterbox";

/// @system Spawns the top/bottom letterbox entities when entering the active state.
pub fn on_letterbox_startup(mut commands: Commands, filters: PxAssets<PxFilter>) {
    #[cfg(debug_assertions)]
    debug_print_startup(DEBUG_MODULE);

    let color = GBColor::Black;

    commands.spawn((
        Name::new("LetterboxTop"),
        LetterboxEntity,
        LetterboxTop,
        PxSubPosition(Vec2::new(0., SCREEN_RESOLUTION_F32.y)),
        PxRectBundle::<Layer> {
            anchor: PxAnchor::BottomLeft,
            canvas: PxCanvas::Camera,
            filter: PxFilter(filters.load_color(color)),
            layers: PxFilterLayers::single_over(Layer::CutsceneLayer(CutsceneLayer::Letterbox)),
            position: PxPosition::from(IVec2::new(0, SCREEN_RESOLUTION.y as i32)),
            rect: PxRect(UVec2::new(SCREEN_RESOLUTION.x, SCREEN_RESOLUTION.y)),
            visibility: Visibility::Visible,
        },
    ));

    commands.spawn((
        Name::new("LetterboxBottom"),
        LetterboxEntity,
        LetterboxBottom,
        PxSubPosition(Vec2::ZERO),
        PxRectBundle::<Layer> {
            anchor: PxAnchor::TopLeft,
            canvas: PxCanvas::Camera,
            filter: PxFilter(filters.load_color(color)),
            layers: PxFilterLayers::single_over(Layer::CutsceneLayer(CutsceneLayer::Letterbox)),
            position: PxPosition::from(IVec2::new(0, 0)),
            rect: PxRect(UVec2::new(SCREEN_RESOLUTION.x, SCREEN_RESOLUTION.y)),
            visibility: Visibility::Visible,
        },
    ));
}

/// @system Cleans up letterbox entities when leaving the active state.
pub fn on_letterbox_shutdown(
    mut commands: Commands,
    entity_query: Query<Entity, With<LetterboxEntity>>,
) {
    #[cfg(debug_assertions)]
    debug_print_shutdown(DEBUG_MODULE);

    mark_for_despawn_by_query(&mut commands, &entity_query);
}

/// @trigger Applies movement instructions to letterbox entities.
pub fn on_move(
    trigger: On<LetterboxMoveEvent>,
    mut commands: Commands,
    top_query: Query<(Entity, &PxSubPosition), With<LetterboxTop>>,
    bottom_query: Query<(Entity, &PxSubPosition), With<LetterboxBottom>>,
) {
    let e = trigger.event();
    for xs in top_query.iter() {
        let target = SCREEN_RESOLUTION_F32.y - e.target;
        insert_linear_movement(&mut commands, xs, target, e.speed);
    }

    for xs in bottom_query.iter() {
        insert_linear_movement(&mut commands, xs, e.target, e.speed);
    }
}

/// Inserts linear movement towards `target`, preserving direction.
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
        // .remove::<LinearValueRemovalBundle<LetterboxTimeDomain, TargetingValueY>>()
        .insert(
            LinearTweenBundle::<LetterboxTimeDomain, TargetingValueY>::new(
                position.y, target, speed,
            ),
        );
}
