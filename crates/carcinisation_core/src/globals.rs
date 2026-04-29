//! Screen-size constants and shared helper functions.

use bevy::{ecs::query::QueryFilter, prelude::*};
use std::sync::LazyLock;

use crate::components::DespawnMark;

/// Native screen resolution (Game Boy: 160x144).
pub const SCREEN_RESOLUTION: UVec2 = UVec2::new(160, 144);

/// Height of the HUD bar at the bottom of the screen.
pub const HUD_HEIGHT: u32 = 14;

/// Default font size for UI text.
pub const FONT_SIZE: u32 = 10;

/// Half screen resolution as `IVec2`.
pub static SCREEN_RESOLUTION_H: LazyLock<IVec2> =
    LazyLock::new(|| (SCREEN_RESOLUTION / 2).as_ivec2());

/// Screen resolution as `Vec2`.
pub static SCREEN_RESOLUTION_F32: LazyLock<Vec2> = LazyLock::new(|| SCREEN_RESOLUTION.as_vec2());

/// Half screen resolution as `Vec2`.
pub static SCREEN_RESOLUTION_F32_H: LazyLock<Vec2> =
    LazyLock::new(|| SCREEN_RESOLUTION.as_vec2() / 2.0);

/// Check if a position is inside an axis-aligned rectangle.
#[must_use]
pub fn is_inside_area(position: Vec2, bottom_left: Vec2, top_right: Vec2) -> bool {
    position.x >= bottom_left.x
        && position.x <= top_right.x
        && position.y >= bottom_left.y
        && position.y <= top_right.y
}

/// Mark all entities matching the query for despawn.
pub fn mark_for_despawn_by_query<F: QueryFilter>(
    commands: &mut Commands,
    query: &Query<'_, '_, Entity, F>,
) {
    for entity in query.iter() {
        commands.entity(entity).insert(DespawnMark);
    }
}

/// Attaches a `DespawnAfterDelay` once a pixel animation finishes.
#[allow(clippy::type_complexity)]
pub fn delay_despawn<D: Default + Send + Sync + 'static>(
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &crate::components::DelayedDespawnOnCxAnimationFinished,
        ),
        (
            With<carapace::prelude::CxAnimationFinished>,
            Without<crate::components::DespawnAfterDelay>,
        ),
    >,
    time: Res<Time<D>>,
) {
    for (entity, delayed) in &mut query.iter_mut() {
        let elapsed = time.elapsed();
        commands
            .entity(entity)
            .insert(crate::components::DespawnAfterDelay {
                elapsed,
                duration: delayed.0,
            });
    }
}

/// Marks entities for despawn once their delay timer expires.
pub fn check_despawn_after_delay<D: Default + Send + Sync + 'static>(
    mut commands: Commands,
    mut query: Query<(Entity, &crate::components::DespawnAfterDelay)>,
    time: Res<Time<D>>,
) {
    for (entity, despawn_after_delay) in &mut query.iter_mut() {
        if despawn_after_delay.elapsed + despawn_after_delay.duration <= time.elapsed() {
            commands.entity(entity).insert(DespawnMark);
        }
    }
}
