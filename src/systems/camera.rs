use bevy::{prelude::*, time::Time};
use leafwing_input_manager::prelude::ActionState;
use seldom_pixel::prelude::{PxCamera, PxSubPosition};

use crate::GBInput;

#[derive(Component)]
pub struct CameraPos;

const CAMERA_SPEED: f32 = 10.;

/**
Move the camera placement using the debug arrow keys
*/
pub fn move_camera(
    mut camera_pos_query: Query<&mut PxSubPosition, With<CameraPos>>,
    gb_input_query: Query<&ActionState<GBInput>>,
    time: Res<Time>,
    mut camera: ResMut<PxCamera>,
) {
    let gb_input = gb_input_query.single();

    let mut camera_pos = camera_pos_query.single_mut();
    **camera_pos += IVec2::new(
        gb_input.pressed(GBInput::DRight) as i32 - gb_input.pressed(GBInput::DLeft) as i32,
        gb_input.pressed(GBInput::DUp) as i32 - gb_input.pressed(GBInput::DDown) as i32,
    )
    .as_vec2()
    .normalize_or_zero()
        * time.delta_seconds()
        * CAMERA_SPEED;

    **camera = camera_pos.round().as_ivec2();
}
