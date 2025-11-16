use crate::plugins::movement::linear::components::*;
use bevy::prelude::*;
use seldom_pixel::prelude::PxSubPosition;

/// @system Syncs TargetingPositionX to PxSubPosition.x
/// Movement children update TargetingPositionX via aggregation, this syncs to the visual position.
pub fn update_position_x(
    mut query: Query<(&TargetingPositionX, &mut PxSubPosition), Changed<TargetingPositionX>>,
) {
    for (targeting_x, mut px_position) in query.iter_mut() {
        px_position.0.x = targeting_x.0;
    }
}

/// @system Syncs TargetingPositionY to PxSubPosition.y
/// Movement children update TargetingPositionY via aggregation, this syncs to the visual position.
pub fn update_position_y(
    mut query: Query<(&TargetingPositionY, &mut PxSubPosition), Changed<TargetingPositionY>>,
) {
    for (targeting_y, mut px_position) in query.iter_mut() {
        px_position.0.y = targeting_y.0;
    }
}
