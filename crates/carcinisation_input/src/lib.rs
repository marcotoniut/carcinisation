//! GB-style input abstraction for Carcinisation.
//!
//! Defines the [`GBInput`] action enum (A, B, D-pad, Start, Select) and its
//! default keyboard mapping. Shared across all game modes (ORS, FPS, menus).

use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

/// Gameboy-style input actions.
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
            GBInput::A => Self::KeyX,
            GBInput::B => Self::ShiftLeft,
            GBInput::Up => Self::ArrowUp,
            GBInput::Down => Self::ArrowDown,
            GBInput::Left => Self::ArrowLeft,
            GBInput::Right => Self::ArrowRight,
            GBInput::Start => Self::Enter,
            GBInput::Select => Self::KeyZ,
            // DEBUG
            GBInput::DUp => Self::KeyW,
            GBInput::DDown => Self::KeyS,
            GBInput::DLeft => Self::KeyA,
            GBInput::DRight => Self::KeyD,
            GBInput::DToGame => Self::KeyI,
            GBInput::DToMainMenu => Self::Delete,
            GBInput::DExit => Self::Escape,
        }
    }
}

/// Spawn the default GB input resources (action state + key map).
pub fn init_gb_input(mut commands: Commands) {
    let mappings: Vec<(GBInput, KeyCode)> = vec![
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
    commands.insert_resource(InputMap::<GBInput>::new(mappings));
}
