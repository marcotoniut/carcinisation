use bevy::{prelude::*, time::Time};
use carapace::prelude::{CxCamera, WorldPos};
use leafwing_input_manager::prelude::ActionState;

use carcinisation_base::game::CameraPos;
use carcinisation_input::GBInput;

const CAMERA_MOVEMENT_SPEED: f32 = 30.;

/// @system DEBUG — moves the camera via debug arrow keys.
pub fn move_camera(
    mut camera_pos_query: Query<&mut WorldPos, With<CameraPos>>,
    gb_input: Res<ActionState<GBInput>>,
    time: Res<Time>,
    mut camera: ResMut<CxCamera>,
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
