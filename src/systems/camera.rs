use bevy::{prelude::*, time::Time};
use seldom_pixel::prelude::{PxCamera, PxSubPosition};

#[derive(Component)]
pub struct CameraPos;

const CAMERA_SPEED: f32 = 10.;

// Move the camera based on the arrow keys
pub fn move_camera(
    mut camera_poses: Query<&mut PxSubPosition, With<CameraPos>>,
    keys: Res<Input<KeyCode>>,
    time: Res<Time>,
    mut camera: ResMut<PxCamera>,
) {
    let mut camera_pos = camera_poses.single_mut();
    **camera_pos += IVec2::new(
        keys.pressed(KeyCode::D) as i32 - keys.pressed(KeyCode::A) as i32,
        keys.pressed(KeyCode::W) as i32 - keys.pressed(KeyCode::S) as i32,
    )
    .as_vec2()
    .normalize_or_zero()
        * time.delta_seconds()
        * CAMERA_SPEED;

    **camera = camera_pos.round().as_ivec2();
}
