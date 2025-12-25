//! Shared Bevy bootstrap for the main game binaries.

use bevy::window::{WindowCloseRequested, WindowResolution};
use bevy::{asset::AssetPlugin, prelude::*};
use bevy_framepace::*;
#[cfg(debug_assertions)]
use bevy_inspector_egui::bevy_egui::EguiPlugin;
#[cfg(debug_assertions)]
use bevy_inspector_egui::{
    bevy_egui::{egui, EguiContext, EguiPrimaryContextPass, PrimaryEguiContext},
    bevy_inspector, DefaultInspectorConfigPlugin,
};
use carcinisation_core::bevy_utils::despawn_entities;
use carcinisation_core::components::{DespawnMark, VolumeSettings};

use crate::{
    cutscene::CutscenePlugin,
    debug::DebugPlugin,
    game::GamePlugin,
    globals::{DEFAULT_CROSSHAIR_INDEX, SCREEN_RESOLUTION, VIEWPORT_RESOLUTION},
    input::GBInput,
    layer::Layer,
    letterbox::LetterboxPlugin,
    main_menu::MainMenuPlugin,
    pixel::PixelPlugin,
    resources::DifficultySelected,
    stage::{player::crosshair::CrosshairSettings, StagePlugin},
    systems::{
        camera::move_camera,
        movement::{update_position_x, update_position_y},
        on_post_startup,
        setup::{init_gb_input, set_fixed_timestep, set_framespace, spawn_camera},
    },
    transitions::spiral::TransitionVenetianPlugin,
};
use leafwing_input_manager::prelude::InputManagerPlugin;
use seldom_pixel::animation::PxAnimationPlugin;
use seldom_pixel::prelude::*;

/// Controls whether the full start/menu stack should run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartFlow {
    Full,
    StageOnly,
}

impl StartFlow {
    const fn includes_start_flow(self) -> bool {
        matches!(self, StartFlow::Full)
    }
}

/// Options for building a Carcinisation `App`.
#[derive(Debug, Clone, Copy)]
pub struct AppLaunchOptions {
    pub start_flow: StartFlow,
}

impl Default for AppLaunchOptions {
    fn default() -> Self {
        Self {
            start_flow: StartFlow::Full,
        }
    }
}

/// Builds the Bevy `App` with shared plugins/resources, parameterised by the entry flow.
pub fn build_app(options: AppLaunchOptions) -> App {
    let title: String = "CARCINISATION".to_string();
    let focused: bool = false;

    let mut app = App::new();
    #[cfg(debug_assertions)]
    {
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
                    close_when_requested: false,
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
                    close_when_requested: false,
                    ..default()
                })
                .set(AssetPlugin {
                    file_path: "../../assets".into(),
                    ..default()
                }),
        );
    }

    app.init_resource::<DifficultySelected>()
        .init_resource::<VolumeSettings>()
        .add_plugins(InputManagerPlugin::<GBInput>::default());

    #[cfg(not(target_arch = "wasm32"))]
    {
        app.add_plugins(FramepacePlugin)
            .add_systems(Startup, (set_framespace, set_fixed_timestep));
    }

    app.add_plugins(PixelPlugin::<Layer>::default())
        .add_systems(Startup, (spawn_camera, init_gb_input))
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(CrosshairSettings(DEFAULT_CROSSHAIR_INDEX))
        .add_plugins(PxPlugin::<Layer>::new(
            SCREEN_RESOLUTION,
            "palette/base.png",
        ))
        .add_plugins(PxAnimationPlugin)
        .add_plugins(TransitionVenetianPlugin)
        .add_plugins(LetterboxPlugin);

    if options.start_flow.includes_start_flow() {
        app.add_plugins(CutscenePlugin)
            .add_plugins(MainMenuPlugin)
            .add_systems(PostStartup, on_post_startup);
    }

    app.add_plugins(StagePlugin)
        .add_plugins(GamePlugin)
        .add_systems(Update, (move_camera, update_position_x, update_position_y))
        .add_systems(Update, exit_on_window_close_request)
        .add_systems(PostUpdate, despawn_entities::<DespawnMark>);

    app
}

/// @system Exits the app when the user requests the window to close.
fn exit_on_window_close_request(
    mut close_requests: MessageReader<WindowCloseRequested>,
    mut exit: MessageWriter<AppExit>,
) {
    if close_requests.read().next().is_some() {
        exit.write(AppExit::Success);
    }
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
