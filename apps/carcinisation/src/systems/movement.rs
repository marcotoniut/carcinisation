use bevy::prelude::*;
use carapace::prelude::WorldPos;
use cween::linear::components::{TargetingValueX, TargetingValueY};

use crate::systems::camera::CameraPos;

/// Systems that sync tween-driven `TargetingValueX`/`Y` into `WorldPos`.
///
/// Any system that reads `WorldPos` after tween integration (e.g.
/// `update_composed_enemy_visuals`) should declare `.after(PositionSyncSystems)`.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct PositionSyncSystems;

/// @system Syncs `TargetingValueX` to WorldPos.x
/// Movement children update `TargetingValueX` via aggregation, this syncs to the visual position.
pub fn update_position_x(
    mut query: Query<
        (&TargetingValueX, &mut WorldPos),
        (Changed<TargetingValueX>, Without<CameraPos>),
    >,
) {
    for (targeting_x, mut px_position) in &mut query {
        px_position.0.x = targeting_x.0;
    }
}

/// @system Syncs `TargetingValueY` to WorldPos.y
/// Movement children update `TargetingValueY` via aggregation, this syncs to the visual position.
pub fn update_position_y(
    mut query: Query<
        (&TargetingValueY, &mut WorldPos),
        (Changed<TargetingValueY>, Without<CameraPos>),
    >,
) {
    for (targeting_y, mut px_position) in &mut query {
        px_position.0.y = targeting_y.0;
    }
}
