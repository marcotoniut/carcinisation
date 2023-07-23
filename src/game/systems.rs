use bevy::prelude::*;

use super::GameState;

pub fn pause_simulation(mut game_state_next_state: ResMut<NextState<GameState>>) {
    game_state_next_state.set(GameState::Paused);
}

pub fn resume_simulation(mut game_state_next_state: ResMut<NextState<GameState>>) {
    game_state_next_state.set(GameState::Running);
}

pub fn toggle_simulation(
    keyboard_input: Res<Input<KeyCode>>,
    state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        if state.get().to_owned() == GameState::Running {
            next_state.set(GameState::Paused);
            println!("Simulation Paused.");
        } else {
            next_state.set(GameState::Running);
            println!("Simulation Running.");
        }
    }
}
