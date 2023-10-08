use crate::game::{events::GameStartupEvent, GamePluginUpdateState};
use bevy::prelude::*;

pub fn on_startup(
    mut event_reader: EventReader<GameStartupEvent>,
    mut game_state_next_state: ResMut<NextState<GamePluginUpdateState>>,
) {
    for _ in event_reader.iter() {
        game_state_next_state.set(GamePluginUpdateState::Active);
    }
}
