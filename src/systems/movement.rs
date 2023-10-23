use crate::{
    plugins::movement::linear::components::*,
    stage::{components::placement::LinearUpdateDisabled, resources::StageTime},
};
use bevy::prelude::*;
use seldom_pixel::prelude::PxSubPosition;

pub fn update_position_x(
    mut query: Query<
        (&TargetingPositionX, &mut PxSubPosition),
        (
            Without<LinearUpdateDisabled>,
            Without<LinearTargetReached<StageTime, TargetingPositionX>>,
        ),
    >,
) {
    for (progress, mut position) in &mut query.iter_mut() {
        position.0.x = progress.0;
    }
}

pub fn update_position_y(
    mut query: Query<
        (&TargetingPositionY, &mut PxSubPosition),
        (
            Without<LinearUpdateDisabled>,
            Without<LinearTargetReached<StageTime, TargetingPositionY>>,
        ),
    >,
) {
    for (progress, mut position) in &mut query.iter_mut() {
        position.0.y = progress.0;
    }
}
