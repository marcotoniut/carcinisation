//! Application entrypoint: wires Bevy, shared plugins, and platform defaults.

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
mod input;
mod layer;
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

use bevy::prelude::*;
use bevy_framepace::*;
use bevy_utils::despawn_entities;
use components::{DespawnMark, VolumeSettings};
use cutscene::CutscenePlugin;
use debug::DebugPlugin;
use game::GamePlugin;
use globals::{DEFAULT_CROSSHAIR_INDEX, SCREEN_RESOLUTION, VIEWPORT_RESOLUTION};
use input::GBInput;
use layer::Layer;
use leafwing_input_manager::prelude::InputManagerPlugin;
use letterbox::LetterboxPlugin;
use main_menu::MainMenuPlugin;
use pixel::PixelPlugin;
use resources::DifficultySelected;
use seldom_pixel::prelude::*;
use stage::{player::crosshair::CrosshairSettings, StagePlugin};
use systems::{
    camera::move_camera,
    movement::{update_position_x, update_position_y},
    setup::{init_gb_input, set_framespace, spawn_camera},
    *,
};

/// Bootstraps the Bevy `App` with game plugins, resources, and editor tooling.
fn main() {
    let title: String = "CARCINISATION".to_string();
    let focused: bool = false;

    let mut app = App::new();
    #[cfg(debug_assertions)]
    {
        // Debug builds keep the window resizable and enable in-engine debugging tools.
        app.add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title,
                    focused,
                    resizable: true,
                    resolution: VIEWPORT_RESOLUTION.into(),
                    ..default()
                }),
                ..default()
            }),
            bevy_editor_pls::EditorPlugin::new(),
            bevy::diagnostic::LogDiagnosticsPlugin::default(),
            DebugPlugin,
        ));
    }
    #[cfg(not(debug_assertions))]
    {
        // Release builds run fullscreen-friendly and skip editor diagnostics.
        app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title,
                focused,
                resizable: false,
                resolution: VIEWPORT_RESOLUTION.into(),
                ..default()
            }),
            ..default()
        }));
    }
    app
        // Core resources initialise difficulty and audio defaults before systems run.
        // TEMP
        // .insert_resource(GlobalVolume::new(0.3))
        .init_resource::<DifficultySelected>()
        .init_resource::<VolumeSettings>()
        // Setup
        // Input, timing, and pixel pipeline plugins come before game-specific plugins.
        .add_plugins(InputManagerPlugin::<GBInput>::default())
        .add_plugins(FramepacePlugin)
        .add_plugins(PixelPlugin::<Layer>::default())
        .add_systems(Startup, (spawn_camera, set_framespace, init_gb_input))
        // Graphics and Game
        // Seed render state and register the core gameplay plugin stack.
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
                // Core camera and positioning updates run every frame.
                move_camera,
                update_position_x,
                update_position_y,
                // transition_to_game_state,
                // transition_to_main_menu_state,
                // input_exit_game,
            ),
        )
        // Cleanup
        // Despawn entities marked for removal after systems finish.
        .add_systems(PostUpdate, despawn_entities::<DespawnMark>)
        .run();
}
