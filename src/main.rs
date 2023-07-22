mod events;
mod game;
mod main_menu;
mod systems;

use bevy::prelude::*;
use game::GamePlugin;
use systems::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "My Bevy Game".to_string(),
                focused: false,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(GamePlugin)
        .add_systems(Startup, spawn_camera)
        .add_systems(Update, exit_game)
        .add_systems(Update, handle_game_over)
        .run();
}
