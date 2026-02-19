use bevy::{prelude::*, time::Time};
use leafwing_input_manager::prelude::ActionState;
use seldom_pixel::prelude::{PxCamera, PxSubPosition};

use crate::input::GBInput;

#[derive(Component)]
pub struct CameraPos;

const CAMERA_MOVEMENT_SPEED: f32 = 30.;

/// @system DEBUG — moves the camera via debug arrow keys.
pub fn move_camera(
    mut camera_pos_query: Query<&mut PxSubPosition, With<CameraPos>>,
    gb_input: Res<ActionState<GBInput>>,
    time: Res<Time>,
    mut camera: ResMut<PxCamera>,
) {
    if let Ok(mut camera_pos) = camera_pos_query.single_mut() {
        **camera_pos += IVec2::new(
            i32::from(gb_input.pressed(&GBInput::DRight))
                - i32::from(gb_input.pressed(&GBInput::DLeft)),
            i32::from(gb_input.pressed(&GBInput::DUp))
                - i32::from(gb_input.pressed(&GBInput::DDown)),
        )
        .as_vec2()
        .normalize_or_zero()
            * time.delta().as_secs_f32()
            * CAMERA_MOVEMENT_SPEED;

        **camera = camera_pos.round().as_ivec2();
    }
}
