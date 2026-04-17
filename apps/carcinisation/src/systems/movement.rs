use bevy::prelude::*;
use carapace::prelude::PxSubPosition;
use cween::linear::components::{TargetingValueX, TargetingValueY};

use crate::systems::camera::CameraPos;

/// Systems that sync tween-driven `TargetingValueX`/`Y` into `PxSubPosition`.
///
/// Any system that reads `PxSubPosition` after tween integration (e.g.
/// `update_composed_enemy_visuals`) should declare `.after(PositionSyncSystems)`.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct PositionSyncSystems;

/// @system Syncs `TargetingValueX` to PxSubPosition.x
/// Movement children update `TargetingValueX` via aggregation, this syncs to the visual position.
pub fn update_position_x(
    mut query: Query<
        (&TargetingValueX, &mut PxSubPosition),
        (Changed<TargetingValueX>, Without<CameraPos>),
    >,
) {
    for (targeting_x, mut px_position) in &mut query {
        px_position.0.x = targeting_x.0;
    }
}

/// @system Syncs `TargetingValueY` to PxSubPosition.y
/// Movement children update `TargetingValueY` via aggregation, this syncs to the visual position.
pub fn update_position_y(
    mut query: Query<
        (&TargetingValueY, &mut PxSubPosition),
        (Changed<TargetingValueY>, Without<CameraPos>),
    >,
) {
    for (targeting_y, mut px_position) in &mut query {
        px_position.0.y = targeting_y.0;
    }
}
