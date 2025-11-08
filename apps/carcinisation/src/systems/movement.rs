use crate::{
    plugins::movement::linear::components::*,
    stage::{components::placement::LinearUpdateDisabled, resources::StageTime},
};
use bevy::prelude::*;
use seldom_pixel::prelude::PxSubPosition;

type AxisQuery<'w, 's, T> = Query<
    'w,
    's,
    (&'static T, &'static mut PxSubPosition),
    (
        Without<LinearUpdateDisabled>,
        Without<LinearTargetReached<StageTime, T>>,
    ),
>;

pub fn update_position_x(mut query: AxisQuery<'_, '_, TargetingPositionX>) {
    for (progress, mut position) in &mut query.iter_mut() {
        position.0.x = progress.0;
    }
}

pub fn update_position_y(mut query: AxisQuery<'_, '_, TargetingPositionY>) {
    for (progress, mut position) in &mut query.iter_mut() {
        position.0.y = progress.0;
    }
}
