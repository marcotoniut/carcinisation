mod assets;
mod bevy_utils;
mod cinemachine;
mod components;
mod core;
mod cutscene;
mod data;
mod game;
mod globals;
mod main_menu;
mod plugins;
mod resource;
mod stage;
mod systems;
mod transitions;

#[macro_use]
extern crate lazy_static;

use crate::globals::{DEFAULT_MASTER_VOLUME, DEFAULT_MUSIC_VOLUME, DEFAULT_SFX_VOLUME};
use bevy::prelude::*;
use bevy_framepace::*;
use bevy_utils::despawn_entities;
use cinemachine::cinemachine::CurrentClipInfo;
use components::DespawnMark;
use game::GamePlugin;
use globals::{DEFAULT_CROSSHAIR_INDEX, SCREEN_RESOLUTION};
use leafwing_input_manager::{
    prelude::{ActionState, InputManagerPlugin},
    Actionlike,
};
use seldom_pixel::prelude::*;
use stage::{player::crosshair::CrosshairSettings, StagePlugin};
use systems::{
    audio::VolumeSettings,
    camera::move_camera,
    setup::{init_gb_input, set_framespace, spawn_camera},
    *,
};
// use transitions::spiral::TransitionVenetianPlugin;

fn main() {
    let title: String = "CARCINISATION".to_string();
    let focused: bool = false;
    // let resolution: Vec2 = Vec2::new(576., 480.);
    let resolution: Vec2 = Vec2::new(850., 480.);

    let mut app = App::new();
    #[cfg(debug_assertions)]
    {
        app.add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title,
                    focused,
                    resizable: true,
                    resolution: (resolution
                        + Vec2::new(
                            SCREEN_RESOLUTION.x as f32 * 2.,
                            SCREEN_RESOLUTION.y as f32 * 1.4,
                        ))
                    .into(),
                    ..default()
                }),
                ..default()
            }),
            bevy_editor_pls::EditorPlugin::new(),
            bevy::diagnostic::LogDiagnosticsPlugin::default(),
        ));
    }
    #[cfg(not(debug_assertions))]
    {
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
    ))
    // TEMP
    // .insert_resource(GlobalVolume::new(0.3))
    .insert_resource(ClearColor(Color::BLACK))
    .insert_resource(CrosshairSettings(DEFAULT_CROSSHAIR_INDEX))
    .insert_resource(VolumeSettings(
        DEFAULT_MASTER_VOLUME,
        DEFAULT_MUSIC_VOLUME,
        DEFAULT_SFX_VOLUME,
    ))
    .insert_resource(CurrentClipInfo {
        index: 0,
        is_rendered: false,
        has_finished: false,
    })
    // .add_plugins(TransitionVenetianPlugin)
    // .add_plugins(CutscenePlugin)
    .add_plugins(StagePlugin)
    .add_plugins(GamePlugin)
    // .add_plugins(MainMenuPlugin)
    .add_plugins(InputManagerPlugin::<GBInput>::default())
    .init_resource::<ActionState<GBInput>>()
    .add_systems(Startup, (spawn_camera, set_framespace, init_gb_input))
    .add_systems(PostStartup, trigger_game_startup)
    .add_systems(PostUpdate, despawn_entities::<DespawnMark>)
    .add_systems(
        Update,
        (
            move_camera,
            // transition_to_game_state,
            // transition_to_main_menu_state,
            // input_exit_game,
        ),
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
    // DEBUG
    DUp,
    DDown,
    DLeft,
    DRight,
    DToGame,
    DToMainMenu,
    DExit,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub enum DepthBase {
    Zero,
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub enum PreBackgroundDepth {
    Zero,
    One,
    Two,
}

#[px_layer]
pub enum Layer {
    Skybox,

    PreDepth(PreBackgroundDepth),
    Background,
    PosDepth(DepthBase),

    Middle(usize),
    Attack,
    #[default]
    Front,
    HudBackground,
    Hud,
    Pickups,
    UIBackground,
    UI,
    CutsceneBackground,
    Cutscene(usize),
    Letterbox,
    CutsceneText,
    Transition,
}
