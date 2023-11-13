use crate::{game::events::GameStartupEvent, main_menu::events::MainMenuShutdownEvent, GBInput};
use bevy::prelude::{EventWriter, Res};
use leafwing_input_manager::prelude::ActionState;

pub fn press_next() {}

pub fn press_esc() {}

pub fn press_start(
    mut game_startup_event_writer: EventWriter<GameStartupEvent>,
    mut main_menu_event_writer: EventWriter<MainMenuShutdownEvent>,
    gb_input: Res<ActionState<GBInput>>,
) {
    if gb_input.pressed(GBInput::Start)
        || gb_input.pressed(GBInput::A)
        || gb_input.pressed(GBInput::B)
    {
        game_startup_event_writer.send(GameStartupEvent);
        main_menu_event_writer.send(MainMenuShutdownEvent);
    }
}
