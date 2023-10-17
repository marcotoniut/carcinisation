use crate::GBInput;
use bevy::prelude::*;
use bevy_framepace::Limiter;
use leafwing_input_manager::prelude::{ActionState, InputMap};
use seldom_pixel::prelude::PxSubPosition;

use super::camera::CameraPos;

pub fn set_framespace(mut settings: ResMut<bevy_framepace::FramepaceSettings>) {
    settings.limiter = Limiter::from_framerate(59.727500569606);
}

pub fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
    commands.spawn((PxSubPosition::default(), CameraPos));
}

pub fn init_gb_input(mut commands: Commands) {
    commands.insert_resource(ActionState::<GBInput>::default());
    commands.insert_resource(InputMap::<GBInput>::new([
        (KeyCode::Left, GBInput::Left),
        (KeyCode::Up, GBInput::Up),
        (KeyCode::Right, GBInput::Right),
        (KeyCode::Down, GBInput::Down),
        (KeyCode::Z, GBInput::B),
        (KeyCode::X, GBInput::A),
        (KeyCode::Return, GBInput::Start),
        (KeyCode::ShiftRight, GBInput::Select),
        // DEBUG
        (KeyCode::I, GBInput::DToGame),
        (KeyCode::Back, GBInput::DToMainMenu),
        (KeyCode::Escape, GBInput::DExit),
        (KeyCode::A, GBInput::DLeft),
        (KeyCode::W, GBInput::DUp),
        (KeyCode::D, GBInput::DRight),
        (KeyCode::S, GBInput::DDown),
    ]));
}
