use bevy::prelude::*;
use cween::linear::components::*;
use seldom_pixel::prelude::PxSubPosition;

use crate::systems::camera::CameraPos;

/// @system Syncs TargetingValueX to PxSubPosition.x
/// Movement children update TargetingValueX via aggregation, this syncs to the visual position.
pub fn update_position_x(
    mut query: Query<
        (&TargetingValueX, &mut PxSubPosition),
        (Changed<TargetingValueX>, Without<CameraPos>),
    >,
) {
    for (targeting_x, mut px_position) in query.iter_mut() {
        px_position.0.x = targeting_x.0;
    }
}

/// @system Syncs TargetingValueY to PxSubPosition.y
/// Movement children update TargetingValueY via aggregation, this syncs to the visual position.
pub fn update_position_y(
    mut query: Query<
        (&TargetingValueY, &mut PxSubPosition),
        (Changed<TargetingValueY>, Without<CameraPos>),
    >,
) {
    for (targeting_y, mut px_position) in query.iter_mut() {
        px_position.0.y = targeting_y.0;
    }
}
