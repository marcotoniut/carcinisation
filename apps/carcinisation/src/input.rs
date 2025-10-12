use bevy::{prelude::KeyCode, reflect::Reflect};
use leafwing_input_manager::Actionlike;

#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug, Reflect)]
pub enum GBInput {
    A,
    B,
    Up,
    Down,
    Left,
    Right,
    Start,
    Select,
    // DEBUG
    DUp,
    DDown,
    DLeft,
    DRight,
    DToGame,
    DToMainMenu,
    DExit,
}

impl From<GBInput> for KeyCode {
    fn from(x: GBInput) -> Self {
        match x {
            GBInput::A => KeyCode::KeyX,
            GBInput::B => KeyCode::KeyZ,
            GBInput::Up => KeyCode::ArrowUp,
            GBInput::Down => KeyCode::ArrowDown,
            GBInput::Left => KeyCode::ArrowLeft,
            GBInput::Right => KeyCode::ArrowRight,
            GBInput::Start => KeyCode::Enter,
            GBInput::Select => KeyCode::ShiftRight,
            // DEBUG
            GBInput::DUp => KeyCode::KeyW,
            GBInput::DDown => KeyCode::KeyS,
            GBInput::DLeft => KeyCode::KeyA,
            GBInput::DRight => KeyCode::KeyD,
            GBInput::DToGame => KeyCode::KeyI,
            GBInput::DToMainMenu => KeyCode::Delete,
            GBInput::DExit => KeyCode::Escape,
        }
    }
}
