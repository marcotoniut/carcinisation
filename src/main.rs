mod assets;
mod events;
// mod game;
// mod main_menu;
mod globals;
mod stage;
mod systems;

use bevy::{prelude::*, window::PrimaryWindow};
use globals::resolution;
use seldom_pixel::prelude::*;
use stage::StagePlugin;
use systems::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "PUNISHED GB".to_string(),
                    focused: false,
                    resizable: false,
                    resolution: Vec2::new(800., 720.).into(),
                    ..default()
                }),
                ..default()
            }),
            PxPlugin::<Layer>::new(resolution, "palette/base.png".into()),
        ))
        .insert_resource(ClearColor(Color::BLACK))
        .add_state::<AppState>()
        .add_plugins(StagePlugin)
        // .add_plugins(MainMenuPlugin)
        .add_systems(Startup, spawn_camera)
        .add_systems(Update, input_exit_game)
        .add_systems(Update, handle_game_over)
        .add_systems(
            Update,
            (transition_to_game_state, transition_to_main_menu_state),
        )
        .run();
}

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
pub enum AppState {
    MainMenu,
    #[default]
    Game,
    GameOver,
}

#[px_layer]
struct Layer;

// #[px_layer]
// enum Layer {
//     #[default]
//     Back,
//     Middle(i32),
//     Front,
// }
