use crate::input::GBInput;

use super::camera::CameraPos;
use bevy::{prelude::*, time::Fixed};
use bevy_framepace::Limiter;
use leafwing_input_manager::prelude::{ActionState, InputMap};
use seldom_pixel::prelude::PxSubPosition;

/// @system Caps the frame limiter to the target Game Boy refresh rate.
pub fn set_framespace(mut settings: ResMut<bevy_framepace::FramepaceSettings>) {
    settings.limiter = Limiter::from_framerate(59.727500569606);
}

/// @system Aligns the fixed timestep with the target framerate.
pub fn set_fixed_timestep(mut fixed_time: ResMut<Time<Fixed>>) {
    fixed_time.set_timestep_hz(59.727500569606);
}

/// @system Spawns the 2D camera and a `CameraPos` tracking entity.
pub fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.spawn((PxSubPosition::default(), CameraPos));
}

/// @system Registers the Game Boy input map and action state resources.
pub fn init_gb_input(mut commands: Commands) {
    let ms: Vec<(GBInput, KeyCode)> = vec![
        (GBInput::Left, GBInput::Left.into()),
        (GBInput::Up, GBInput::Up.into()),
        (GBInput::Right, GBInput::Right.into()),
        (GBInput::Down, GBInput::Down.into()),
        (GBInput::B, GBInput::B.into()),
        (GBInput::A, GBInput::A.into()),
        (GBInput::Start, GBInput::Start.into()),
        (GBInput::Select, GBInput::Select.into()),
        // DEBUG
        (GBInput::DToGame, GBInput::DToGame.into()),
        (GBInput::DToMainMenu, GBInput::DToMainMenu.into()),
        (GBInput::DExit, GBInput::DExit.into()),
        (GBInput::DLeft, GBInput::DLeft.into()),
        (GBInput::DUp, GBInput::DUp.into()),
        (GBInput::DRight, GBInput::DRight.into()),
        (GBInput::DDown, GBInput::DDown.into()),
    ];
    commands.insert_resource(ActionState::<GBInput>::default());
    commands.insert_resource(InputMap::<GBInput>::new(ms));
}
