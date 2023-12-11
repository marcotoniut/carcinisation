use super::camera::CameraPos;
use crate::GBInput;
use bevy::prelude::*;
use bevy_framepace::Limiter;
use leafwing_input_manager::prelude::{ActionState, InputMap};
use seldom_pixel::prelude::PxSubPosition;

pub fn set_framespace(mut settings: ResMut<bevy_framepace::FramepaceSettings>) {
    settings.limiter = Limiter::from_framerate(59.727500569606);
}

pub fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
    commands.spawn((PxSubPosition::default(), CameraPos));
}

pub fn init_gb_input(mut commands: Commands) {
    let ys: Vec<(KeyCode, GBInput)> = vec![
        (GBInput::Left.into(), GBInput::Left),
        (GBInput::Up.into(), GBInput::Up),
        (GBInput::Right.into(), GBInput::Right),
        (GBInput::Down.into(), GBInput::Down),
        (GBInput::B.into(), GBInput::B),
        (GBInput::A.into(), GBInput::A),
        (GBInput::Start.into(), GBInput::Start),
        (GBInput::Select.into(), GBInput::Select),
        // DEBUG
        (GBInput::DToGame.into(), GBInput::DToGame),
        (GBInput::DToMainMenu.into(), GBInput::DToMainMenu),
        (GBInput::DExit.into(), GBInput::DExit),
        (GBInput::DLeft.into(), GBInput::DLeft),
        (GBInput::DUp.into(), GBInput::DUp),
        (GBInput::DRight.into(), GBInput::DRight),
        (GBInput::DDown.into(), GBInput::DDown),
    ];
    commands.insert_resource(ActionState::<GBInput>::default());
    commands.insert_resource(InputMap::<GBInput>::new(ys));
}
