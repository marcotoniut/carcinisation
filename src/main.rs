mod assets;
mod cutscene;
mod events;
mod globals;
mod main_menu;
mod stage;
mod systems;
mod transitions;
mod entities;
mod ui;

use bevy::{prelude::*, window::{PresentMode, WindowResolution}};
use bevy_framepace::*;
use cutscene::CutscenePlugin;
use globals::SCREEN_RESOLUTION;
use leafwing_input_manager::{prelude::InputManagerPlugin, Actionlike};
use seldom_pixel::prelude::*;
use stage::StagePlugin;
use systems::{camera::move_camera, *};
use ui::{render_crosshair, crosshair::CrosshairSettings, move_crosshair};
// use transitions::spiral::TransitionVenetianPlugin;

fn main() {
    let title: String = "PUNISHED GB".to_string();
    let focused: bool = false;
    //let resolution: Vec2 = Vec2::new(800., 720.);
    let resolution = Vec2::new(globals::SCREEN_RESOLUTION.x as f32, globals::SCREEN_RESOLUTION.y as f32);

    let mut app = App::new();
    let dev = true;
    if dev {
        app.add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title,
                    focused,
                    resizable: true,
                    resolution: WindowResolution::new(resolution.x, resolution.y).with_scale_factor_override(1.0),
                    ..default()
                }),
                ..default()
            }),
            bevy_editor_pls::prelude::EditorPlugin::new(),
        ));
    } else {
        app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title,
                focused,
                resizable: false,
                resolution: resolution.into(),
                ..default()
            }),
            ..default()
        }));
    }
    app.add_plugins((
        PxPlugin::<Layer>::new(SCREEN_RESOLUTION, "palette/base.png".into()),
        FramepacePlugin,
        bevy::diagnostic::LogDiagnosticsPlugin::default(),
    ))
    // .insert_resource(GlobalVolume::new(0.2))
    .insert_resource(ClearColor(Color::BLACK))
    .insert_resource(CrosshairSettings(0))
    .add_state::<AppState>()
    // .add_plugins(TransitionVenetianPlugin)
    .add_plugins(CutscenePlugin)
    // .add_plugins(StagePlugin)
    // .add_plugins(MainMenuPlugin)
    .add_plugins(InputManagerPlugin::<GBInput>::default())
    .add_systems(Startup, (set_framespace, spawn_camera, spawn_gb_input))
    // .add_systems(Update, move_camera)
    // .add_systems(Update, input_exit_game)
    // // TODO should this be placed at main?
    // // .add_systems(Update, handle_game_over)
    // .add_systems(
    //     Update,
    //     (transition_to_game_state, transition_to_main_menu_state),
    // )
    .add_systems(Startup,render_crosshair)
    .add_systems(Update, move_crosshair)
    .run();
}

// This is the list of "things in the game I want to be able to do based on input"
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
    // debug inputs
    DUp,
    DDown,
    DLeft,
    DRight,
    DToGame,
    DToMainMenu,
    DExit,
}

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
pub enum AppState {
    #[default]
    Cutscene,
    Transition,
    MainMenu,
    Game,
    GameOver,
}

#[px_layer]
pub enum Layer {
    #[default]
    Back,
    Middle(i32),
    Front,
    UIBackground,
    UI,
    Cutscene(i32),
    Letterbox,
    CutsceneText,
    Transition,
}

pub fn set_window_changes(mut windows: Query<&mut Window>) {
    let window = windows.get_single().unwrap();
}

pub fn resolution_change(mut windows: Query<&mut Window>) {

}
