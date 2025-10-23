//! Application entrypoint: wires Bevy, shared plugins, and platform defaults.

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

use bevy::window::WindowResolution;
use bevy::{asset::AssetPlugin, prelude::*};
use bevy_framepace::*;
#[cfg(debug_assertions)]
use bevy_inspector_egui::bevy_egui::EguiPlugin;
#[cfg(debug_assertions)]
use bevy_inspector_egui::{
    bevy_egui::{egui, EguiContext, EguiPrimaryContextPass, PrimaryEguiContext},
    bevy_inspector, DefaultInspectorConfigPlugin,
};
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
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title,
                        focused,
                        resizable: true,
                        resolution: WindowResolution::new(
                            VIEWPORT_RESOLUTION.x as u32,
                            VIEWPORT_RESOLUTION.y as u32,
                        ),
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    file_path: "../../assets".into(),
                    ..default()
                }),
            EguiPlugin::default(),
            DebugInspectorOverlayPlugin,
            bevy::diagnostic::LogDiagnosticsPlugin::default(),
            DebugPlugin,
        ));
    }
    #[cfg(not(debug_assertions))]
    {
        // Release builds run fullscreen-friendly and skip editor diagnostics.
        app.add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title,
                        focused,
                        resizable: false,
                        resolution: WindowResolution::new(
                            VIEWPORT_RESOLUTION.x as u32,
                            VIEWPORT_RESOLUTION.y as u32,
                        ),
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    file_path: "../../assets".into(),
                    ..default()
                }),
        );
    }
    app
        // Core resources initialise difficulty and audio defaults before systems run.
        // TEMP
        // .insert_resource(GlobalVolume::Linear(0.3))
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
            "palette/base.png",
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

#[cfg(debug_assertions)]
const INSPECTOR_EDGE_PADDING: f32 = 12.0;
#[cfg(debug_assertions)]
const INSPECTOR_TOGGLE_HEIGHT: f32 = 36.0;
#[cfg(debug_assertions)]
const INSPECTOR_DEFAULT_SIZE: egui::Vec2 = egui::Vec2::new(360.0, 440.0);

#[cfg(debug_assertions)]
#[derive(Resource, Default)]
struct InspectorUiState {
    open: bool,
}

#[cfg(debug_assertions)]
struct DebugInspectorOverlayPlugin;

#[cfg(debug_assertions)]
impl Plugin for DebugInspectorOverlayPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<DefaultInspectorConfigPlugin>() {
            app.add_plugins(DefaultInspectorConfigPlugin);
        }

        app.init_resource::<InspectorUiState>();

        app.add_systems(
            EguiPrimaryContextPass,
            inspector_overlay_world_ui.into_configs(),
        );
    }
}

#[cfg(debug_assertions)]
fn inspector_overlay_world_ui(world: &mut World) {
    let toggled_via_key = world
        .get_resource::<ButtonInput<KeyCode>>()
        .map(|keys| keys.just_pressed(KeyCode::F12))
        .unwrap_or(false);

    let mut open = {
        let mut state = world.resource_mut::<InspectorUiState>();
        if toggled_via_key {
            state.open = !state.open;
        }
        state.open
    };

    let Ok(egui_context) = world
        .query_filtered::<&mut EguiContext, With<PrimaryEguiContext>>()
        .single(world)
    else {
        return;
    };

    let mut egui_context = egui_context.clone();
    let ctx = egui_context.get_mut();

    egui::Area::new(egui::Id::new("world_inspector_toggle"))
        .anchor(
            egui::Align2::RIGHT_TOP,
            egui::Vec2::new(-INSPECTOR_EDGE_PADDING, INSPECTOR_EDGE_PADDING),
        )
        .order(egui::Order::Foreground)
        .interactable(true)
        .movable(false)
        .show(ctx, |ui| {
            let label = if open {
                "Hide Inspector"
            } else {
                "Show Inspector"
            };
            if ui.button(label).clicked() {
                open = !open;
            }
        });

    if !open {
        let mut state = world.resource_mut::<InspectorUiState>();
        state.open = open;
        return;
    }

    egui::Window::new("World Inspector")
        .anchor(
            egui::Align2::RIGHT_TOP,
            egui::Vec2::new(
                -INSPECTOR_EDGE_PADDING,
                INSPECTOR_EDGE_PADDING + INSPECTOR_TOGGLE_HEIGHT,
            ),
        )
        .default_width(INSPECTOR_DEFAULT_SIZE.x)
        .default_height(INSPECTOR_DEFAULT_SIZE.y)
        .resizable(true)
        .open(&mut open)
        .show(ctx, |ui| {
            egui::ScrollArea::both().show(ui, |ui| {
                bevy_inspector::ui_for_world(world, ui);
                ui.allocate_space(ui.available_size());
            });
        });

    let mut state = world.resource_mut::<InspectorUiState>();
    state.open = open;
}
