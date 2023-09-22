mod assets;
mod cutscene;
mod events;
mod game;
mod globals;
mod main_menu;
mod resource;
mod stage;
mod systems;
mod transitions;

#[macro_use]
extern crate lazy_static;

use crate::globals::{DEFAULT_MASTER_VOLUME, DEFAULT_MUSIC_VOLUME, DEFAULT_SFX_VOLUME};
use bevy::prelude::*;
use bevy_framepace::*;
use cutscene::CutscenePlugin;
use globals::{DEFAULT_CROSSHAIR_INDEX, SCREEN_RESOLUTION};
use leafwing_input_manager::{prelude::InputManagerPlugin, Actionlike};
use seldom_pixel::prelude::*;
use stage::{data::StageData, player::crosshair::CrosshairSettings, StagePlugin};
use systems::{audio::VolumeSettings, camera::move_camera, *};
// use transitions::spiral::TransitionVenetianPlugin;

fn main() {
    let title: String = "CARCINISATION".to_string();
    let focused: bool = false;
    let resolution: Vec2 = Vec2::new(576., 480.);

    let mut app = App::new();
    let dev = true;
    if dev {
        app.add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title,
                    focused,
                    resizable: true,
                    resolution: (resolution
                        + Vec2::new(
                            SCREEN_RESOLUTION.x as f32 * 1.5,
                            SCREEN_RESOLUTION.y as f32 * 1.5,
                        ))
                    .into(),
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
    .insert_resource(CrosshairSettings(DEFAULT_CROSSHAIR_INDEX))
    .insert_resource(VolumeSettings(
        DEFAULT_MASTER_VOLUME,
        DEFAULT_MUSIC_VOLUME,
        DEFAULT_SFX_VOLUME,
    ))
    .add_state::<AppState>()
    // .add_plugins(TransitionVenetianPlugin)
    // .add_plugins(CutscenePlugin)
    .add_plugins(StagePlugin)
    // .add_plugins(GamePlugin)
    // .add_plugins(MainMenuPlugin)
    .add_plugins(InputManagerPlugin::<GBInput>::default())
    .add_systems(Startup, (set_framespace, spawn_camera, spawn_gb_input))
    .add_systems(Update, move_camera)
    .add_systems(Update, input_exit_game)
    .add_systems(Update, input_snd_menu)
    // TODO should this be placed at main?
    // .add_systems(Update, handle_game_over)
    .add_systems(
        Update,
        (transition_to_game_state, transition_to_main_menu_state),
    )
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
    Cutscene,
    Transition,
    MainMenu,
    #[default]
    Game,
    GameOver,
}

#[px_layer]
pub enum Layer {
    Skybox,
    Back,
    Middle(usize),
    Attack,
    #[default]
    Front,
    UIBackground,
    UI,
    Cutscene(usize),
    Letterbox,
    CutsceneText,
    Transition,
}
