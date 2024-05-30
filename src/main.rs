#![feature(step_trait)]

mod assets;
mod bevy_utils;
mod components;
mod core;
mod cutscene;
mod data;
mod debug;
mod game;
mod globals;
mod letterbox;
mod main_menu;
mod pixel;
mod plugins;
mod progression;
mod resources;
mod stage;
mod systems;
mod transitions;

#[macro_use]
extern crate lazy_static;

use crate::globals::{DEFAULT_MASTER_VOLUME, DEFAULT_MUSIC_VOLUME, DEFAULT_SFX_VOLUME};
use bevy::prelude::*;
use bevy_framepace::*;
use bevy_utils::despawn_entities;
use components::DespawnMark;
use cutscene::{data::CutsceneLayer, CutscenePlugin};
use debug::DebugPlugin;
use game::GamePlugin;
use globals::{DEFAULT_CROSSHAIR_INDEX, SCREEN_RESOLUTION, VIEWPORT_RESOLUTION};
use leafwing_input_manager::{prelude::InputManagerPlugin, Actionlike};
use letterbox::LetterboxPlugin;
use main_menu::MainMenuPlugin;
use pixel::PixelPlugin;
use resources::DifficultySelected;
use seldom_pixel::prelude::*;
use stage::{player::crosshair::CrosshairSettings, StagePlugin};
use systems::{
    audio::VolumeSettings,
    camera::move_camera,
    movement::{update_position_x, update_position_y},
    setup::{init_gb_input, set_framespace, spawn_camera},
    *,
};

fn main() {
    let title: String = "CARCINISATION".to_string();
    let focused: bool = false;

    let mut app = App::new();
    #[cfg(debug_assertions)]
    {
        app.add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title,
                    focused,
                    resizable: true,
                    resolution: VIEWPORT_RESOLUTION.into(),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            bevy_editor_pls::EditorPlugin::new(),
            bevy::diagnostic::LogDiagnosticsPlugin::default(),
            DebugPlugin,
        ));
    }
    #[cfg(not(debug_assertions))]
    {
        app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title,
                focused,
                resizable: false,
                resolution: VIEWPORT_RESOLUTION.into(),
                ..Default::default()
            }),
            ..Default::default()
        }));
    }
    app
        // TEMP
        // .insert_resource(GlobalVolume::new(0.3))
        .init_resource::<DifficultySelected>()
        .insert_resource(VolumeSettings(
            DEFAULT_MASTER_VOLUME,
            DEFAULT_MUSIC_VOLUME,
            DEFAULT_SFX_VOLUME,
        ))
        // Setup
        .add_plugins(InputManagerPlugin::<GBInput>::default())
        .add_plugins(FramepacePlugin)
        .add_plugins(PixelPlugin::<Layer>::default())
        .add_systems(Startup, (spawn_camera, set_framespace, init_gb_input))
        // Graphics and Game
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(CrosshairSettings(DEFAULT_CROSSHAIR_INDEX))
        .add_plugins(PxPlugin::<Layer>::new(
            SCREEN_RESOLUTION,
            "palette/base.png".into(),
        ))
        // .add_plugins(TransitionVenetianPlugin)
        .add_plugins(CutscenePlugin)
        .add_plugins(LetterboxPlugin)
        .add_plugins(MainMenuPlugin)
        .add_plugins(StagePlugin)
        .add_plugins(GamePlugin)
        // .add_systems(PostStartup, debug_trigger_game_startup)
        .add_systems(PostStartup, on_post_startup)
        .add_systems(
            Update,
            (
                move_camera,
                update_position_x,
                update_position_y,
                // transition_to_game_state,
                // transition_to_main_menu_state,
                // input_exit_game,
            ),
        )
        // Cleanup
        .add_systems(PostUpdate, despawn_entities::<DespawnMark>)
        .run();
}

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
            GBInput::A => KeyCode::KeyX,
            GBInput::B => KeyCode::KeyZ,
            GBInput::Up => KeyCode::ArrowUp,
            GBInput::Down => KeyCode::ArrowDown,
            GBInput::Left => KeyCode::ArrowLeft,
            GBInput::Right => KeyCode::ArrowRight,
            GBInput::Start => KeyCode::Enter,
            GBInput::Select => KeyCode::ShiftRight,
            // DEBUG
            GBInput::DUp => KeyCode::KeyW,
            GBInput::DDown => KeyCode::KeyS,
            GBInput::DLeft => KeyCode::KeyA,
            GBInput::DRight => KeyCode::KeyD,
            GBInput::DToGame => KeyCode::KeyI,
            GBInput::DToMainMenu => KeyCode::Delete,
            GBInput::DExit => KeyCode::Escape,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Reflect)]
pub enum MidDepth {
    Six,
    Five,
    Four,
    Three,
    Two,
    One,
    Zero,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Reflect)]
pub enum PreBackgroundDepth {
    Nine,
    Eight,
    Seven,
}

#[derive(Reflect)]
#[px_layer]
pub enum Layer {
    Skybox,

    PreBackgroundDepth(PreBackgroundDepth),
    Background,
    MidDepth(MidDepth),

    Attack,
    #[default]
    Front,
    HudBackground,
    Hud,
    Pickups,
    UIBackground,
    UI,
    CutsceneLayer(CutsceneLayer),
    // CutsceneBackground,
    // Cutscene(u8),
    // Letterbox,
    // CutsceneText,
    Transition,
}

// impl Layer {
//     pub fn get_from_depth(entity_type: StageEntityType, depth: DepthBase) -> Self {}
// }
