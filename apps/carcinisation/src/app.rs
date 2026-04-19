//! Shared Bevy bootstrap for the main game binaries.

#![allow(clippy::too_many_lines)]

#[cfg(not(target_arch = "wasm32"))]
use std::env;

use bevy::render::RenderPlugin;
use bevy::render::settings::{RenderCreation, WgpuSettings};
use bevy::window::{WindowCloseRequested, WindowResolution};
use bevy::{asset::AssetMetaCheck, asset::AssetPlugin, prelude::*};
#[cfg(feature = "brp")]
use bevy_brp_extras::BrpExtrasPlugin;
use bevy_framepace::FramepacePlugin;
#[cfg(debug_assertions)]
use bevy_inspector_egui::bevy_egui::EguiPlugin;
#[cfg(debug_assertions)]
use bevy_inspector_egui::{
    DefaultInspectorConfigPlugin,
    bevy_egui::{EguiContext, EguiPrimaryContextPass, PrimaryEguiContext, egui},
    bevy_inspector,
};
use carcinisation_core::bevy_utils::despawn_entities;
use carcinisation_core::components::{DespawnMark, VolumeSettings};
#[cfg(not(target_arch = "wasm32"))]
use dotenvy::dotenv_override;

#[cfg(debug_assertions)]
use crate::debug::{DebugColliderOverlay, DebugGodMode, DebugPlugin};
#[cfg(feature = "gallery")]
use crate::gallery::GalleryPlugin;
use crate::{
    cutscene::CutscenePlugin,
    game::GamePlugin,
    globals::{ASSETS_PATH, DEFAULT_CROSSHAIR_INDEX, SCREEN_RESOLUTION, VIEWPORT_RESOLUTION},
    input::GBInput,
    layer::Layer,
    letterbox::LetterboxPlugin,
    main_menu::MainMenuPlugin,
    resources::DifficultySelected,
    stage::{StagePlugin, depth_debug::DepthDebugOverlay, player::crosshair::CrosshairSettings},
    systems::{
        camera::move_camera,
        movement::{PositionSyncSystems, update_position_x, update_position_y},
        on_post_startup,
        setup::{init_gb_input, set_fixed_timestep, set_framespace, spawn_camera},
    },
    transitions::spiral::TransitionVenetianPlugin,
};
use carapace::animation::PxAnimationPlugin;
use carapace::prelude::*;
use leafwing_input_manager::prelude::InputManagerPlugin;

/// Controls whether the full start/menu stack should run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartFlow {
    Full,
    StageOnly,
    Gallery,
}

const INITIAL_SOUND_LEVEL_ENV: &str = "CARCINISATION_INITIAL_SOUND";
#[cfg(not(target_arch = "wasm32"))]
const INITIAL_GOD_MODE_ENV: &str = "CARCINISATION_GOD_MODE";
#[cfg(not(target_arch = "wasm32"))]
const SKIP_MENU_ENV: &str = "CARCINISATION_SKIP_MENU";
#[cfg(not(target_arch = "wasm32"))]
const SKIP_CUTSCENES_ENV: &str = "CARCINISATION_SKIP_CUTSCENES";
#[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
const SHOW_COLLIDERS_ENV: &str = "CARCINISATION_SHOW_COLLIDERS";
#[cfg(not(target_arch = "wasm32"))]
const SHOW_PERSPECTIVE_ENV: &str = "CARCINISATION_SHOW_PERSPECTIVE";

impl StartFlow {
    const fn includes_start_flow(self) -> bool {
        matches!(self, StartFlow::Full)
    }

    const fn includes_gallery(self) -> bool {
        matches!(self, StartFlow::Gallery)
    }
}

/// Options for building a Carcinisation `App`.
#[derive(Debug, Clone, Copy)]
pub struct AppLaunchOptions {
    pub start_flow: StartFlow,
    /// When `true`, uses `MinimalPlugins` instead of `DefaultPlugins` and skips
    /// rendering, audio, and window systems. Useful for integration tests.
    pub headless: bool,
}

impl Default for AppLaunchOptions {
    fn default() -> Self {
        Self {
            start_flow: StartFlow::Full,
            headless: false,
        }
    }
}

/// Builds the Bevy `App` with shared plugins/resources, parameterised by the entry flow.
pub fn build_app(options: AppLaunchOptions) -> App {
    let mut app = App::new();

    if options.headless {
        app.add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: None,
                    close_when_requested: false,
                    ..default()
                })
                .set(RenderPlugin {
                    render_creation: RenderCreation::Automatic(WgpuSettings {
                        backends: None,
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    file_path: ASSETS_PATH.into(),
                    meta_check: AssetMetaCheck::Never,
                    ..default()
                })
                .disable::<bevy::winit::WinitPlugin>(),
        );
    } else {
        let title: String = "CARCINISATION".to_string();
        let focused: bool = false;

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
                        file_path: ASSETS_PATH.into(),
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
                        file_path: ASSETS_PATH.into(),
                        ..default()
                    }),
            );
        }
    }

    app.init_resource::<DifficultySelected>()
        .insert_resource(initial_volume_settings())
        .insert_resource(load_dev_flags())
        .add_plugins(InputManagerPlugin::<GBInput>::default());

    app.insert_resource(DepthDebugOverlay::new(load_show_perspective()));

    #[cfg(debug_assertions)]
    {
        app.insert_resource(DebugGodMode::new(load_initial_god_mode(options.start_flow)));
        app.insert_resource(DebugColliderOverlay::new(load_show_colliders()));
    }

    if !options.headless {
        #[cfg(feature = "brp")]
        app.add_plugins(BrpExtrasPlugin);

        #[cfg(not(target_arch = "wasm32"))]
        {
            app.add_plugins(FramepacePlugin)
                .add_systems(Startup, (set_framespace, set_fixed_timestep));
        }
    }

    app.insert_resource(ClearColor(Color::BLACK))
        .insert_resource(CrosshairSettings(DEFAULT_CROSSHAIR_INDEX))
        .add_plugins(PxAnimationPlugin)
        .add_plugins(TransitionVenetianPlugin)
        .add_plugins(LetterboxPlugin);

    let px_plugin = PxPlugin::<Layer>::new(SCREEN_RESOLUTION, "palette/base.png");
    if options.headless {
        px_plugin.build_headless(&mut app);
    } else {
        app.add_plugins(px_plugin);
    }

    if !options.headless {
        app.add_systems(Startup, spawn_camera);
    }

    app.add_systems(Startup, init_gb_input);

    if options.start_flow.includes_start_flow() {
        app.add_plugins(CutscenePlugin)
            .add_plugins(MainMenuPlugin)
            .add_systems(PostStartup, on_post_startup);
    }

    app.configure_sets(Update, PositionSyncSystems);
    app.add_plugins(StagePlugin)
        .add_plugins(GamePlugin)
        .add_systems(
            Update,
            (
                move_camera,
                (update_position_x, update_position_y).in_set(PositionSyncSystems),
            ),
        )
        // NOTE:
        // Systems in PostUpdate that mutate gameplay entities via Commands must
        // avoid entities marked with `DespawnMark`.
        //
        // Reason:
        // `despawn_entities::<DespawnMark>` runs in the same schedule. Deferred
        // inserts targeting entities that are despawned in the same frame will
        // panic.
        //
        // Pattern:
        // - Use `Without<DespawnMark>` in queries
        // - Or use a defensive insert (e.g. try_insert) if appropriate
        .add_systems(PostUpdate, despawn_entities::<DespawnMark>);

    #[cfg(feature = "gallery")]
    if options.start_flow.includes_gallery() || options.start_flow.includes_start_flow() {
        app.add_plugins(GalleryPlugin);
    }

    if !options.headless {
        app.add_systems(Update, exit_on_window_close_request);
    }

    app
}

fn initial_volume_settings() -> VolumeSettings {
    let Some(initial_sound_level) = load_initial_sound_level() else {
        return VolumeSettings::default();
    };

    VolumeSettings::default().with_master_level(initial_sound_level)
}

#[cfg(target_arch = "wasm32")]
fn load_initial_sound_level() -> Option<f32> {
    None
}

#[cfg(not(target_arch = "wasm32"))]
fn load_initial_sound_level() -> Option<f32> {
    let _ = dotenv_override();

    match env::var(INITIAL_SOUND_LEVEL_ENV) {
        Ok(value) => match parse_normalized_sound_level(&value) {
            Ok(level) => Some(level),
            Err(message) => {
                warn!("{INITIAL_SOUND_LEVEL_ENV} {message}; using default audio levels");
                None
            }
        },
        Err(env::VarError::NotPresent) => None,
        Err(env::VarError::NotUnicode(_)) => {
            warn!("{INITIAL_SOUND_LEVEL_ENV} must be valid UTF-8; using default audio levels");
            None
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn load_initial_god_mode(start_flow: StartFlow) -> bool {
    matches!(start_flow, StartFlow::StageOnly)
}

#[cfg(not(target_arch = "wasm32"))]
fn load_initial_god_mode(start_flow: StartFlow) -> bool {
    let _ = dotenv_override();

    match env::var(INITIAL_GOD_MODE_ENV) {
        Ok(value) => match parse_bool_flag(&value) {
            Ok(enabled) => enabled,
            Err(message) => {
                warn!("{INITIAL_GOD_MODE_ENV} {message}; defaulting based on launch flow");
                matches!(start_flow, StartFlow::StageOnly)
            }
        },
        Err(env::VarError::NotPresent) => matches!(start_flow, StartFlow::StageOnly),
        Err(env::VarError::NotUnicode(_)) => {
            warn!("{INITIAL_GOD_MODE_ENV} must be valid UTF-8; defaulting based on launch flow");
            matches!(start_flow, StartFlow::StageOnly)
        }
    }
}

#[cfg(all(debug_assertions, target_arch = "wasm32"))]
fn load_show_colliders() -> bool {
    false
}

#[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
fn load_show_colliders() -> bool {
    let _ = dotenv_override();
    env::var(SHOW_COLLIDERS_ENV)
        .ok()
        .and_then(|v| parse_bool_flag(&v).ok())
        .unwrap_or(false)
}

#[cfg(target_arch = "wasm32")]
fn load_show_perspective() -> bool {
    false
}

#[cfg(not(target_arch = "wasm32"))]
fn load_show_perspective() -> bool {
    let _ = dotenv_override();
    env::var(SHOW_PERSPECTIVE_ENV)
        .ok()
        .and_then(|v| parse_bool_flag(&v).ok())
        .unwrap_or(false)
}

#[cfg(target_arch = "wasm32")]
fn load_dev_flags() -> crate::resources::DevFlags {
    crate::resources::DevFlags::default()
}

#[cfg(not(target_arch = "wasm32"))]
fn load_dev_flags() -> crate::resources::DevFlags {
    let _ = dotenv_override();

    let skip_menu = env::var(SKIP_MENU_ENV)
        .ok()
        .and_then(|v| parse_bool_flag(&v).ok())
        .unwrap_or(false);
    let skip_cutscenes = env::var(SKIP_CUTSCENES_ENV)
        .ok()
        .and_then(|v| parse_bool_flag(&v).ok())
        .unwrap_or(false);

    crate::resources::DevFlags {
        skip_menu,
        skip_cutscenes,
    }
}

fn parse_normalized_sound_level(value: &str) -> Result<f32, &'static str> {
    let level = value
        .trim()
        .parse::<f32>()
        .map_err(|_| "must parse as a number between 0.0 and 1.0")?;

    if !(0.0..=1.0).contains(&level) {
        return Err("must be between 0.0 and 1.0");
    }

    Ok(level)
}

fn parse_bool_flag(value: &str) -> Result<bool, &'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err("must be one of 1/0/true/false/yes/no/on/off"),
    }
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

#[cfg(test)]
mod tests {
    use super::{parse_bool_flag, parse_normalized_sound_level};

    #[test]
    #[allow(clippy::float_cmp)]
    fn accepts_values_inside_range() {
        assert_eq!(parse_normalized_sound_level("0").unwrap(), 0.0);
        assert_eq!(parse_normalized_sound_level("0.5").unwrap(), 0.5);
        assert_eq!(parse_normalized_sound_level("1").unwrap(), 1.0);
    }

    #[test]
    fn rejects_values_outside_range() {
        assert!(parse_normalized_sound_level("-0.1").is_err());
        assert!(parse_normalized_sound_level("1.1").is_err());
    }

    #[test]
    fn rejects_non_numeric_values() {
        assert!(parse_normalized_sound_level("loud").is_err());
    }

    #[test]
    fn parses_common_bool_flags() {
        assert!(parse_bool_flag("true").unwrap());
        assert!(parse_bool_flag("On").unwrap());
        assert!(!parse_bool_flag("false").unwrap());
        assert!(!parse_bool_flag("0").unwrap());
    }

    #[test]
    fn rejects_unknown_bool_flags() {
        assert!(parse_bool_flag("maybe").is_err());
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
        .is_some_and(|keys| keys.just_pressed(KeyCode::F12));

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
