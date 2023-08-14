use bevy::{app::AppExit, prelude::*, window::PrimaryWindow};

use crate::{events::*, AppState};

pub fn input_exit_game(keyboard_input: Res<Input<KeyCode>>, mut exit: ResMut<Events<AppExit>>) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        exit.send(AppExit);
    }
}

pub fn handle_game_over(mut game_over_event_reader: EventReader<GameOver>) {
    for game_over in game_over_event_reader.iter() {
        println!("Your final score: {}", game_over.score);
    }
}

pub fn spawn_camera(mut commands: Commands, window_query: Query<&Window, With<PrimaryWindow>>) {
    let window: &Window = window_query.get_single().unwrap();
    // commands.spawn(Camera2dBundle {
    //     transform: Transform::from_xyz(window.width() / 2.0, window.height() / 2.0, 10.0),
    //     ..default()
    // });
    commands.spawn(Camera2dBundle::default());
}

pub fn transition_to_game_state(
    keyboard_input: Res<Input<KeyCode>>,
    app_state: Res<State<AppState>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if keyboard_input.just_pressed(KeyCode::G) {
        if app_state.get().to_owned() != AppState::Game {
            next_state.set(AppState::Game);
            println!("Entered AppState::Game");
        }
    }
}

pub fn transition_to_main_menu_state(
    keyboard_input: Res<Input<KeyCode>>,
    app_state: Res<State<AppState>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if keyboard_input.just_pressed(KeyCode::M) {
        if app_state.get().to_owned() != AppState::MainMenu {
            // commands.insert_resource(NextState(Some(AppState::MainMenu)));
            next_state.set(AppState::MainMenu);
            println!("Entered AppState::MainMenu");
        }
    }
}
