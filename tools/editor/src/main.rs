#[cfg(feature = "full_editor")]
mod builders;
#[cfg(feature = "full_editor")]
mod components;
#[cfg(feature = "full_editor")]
mod constants;
#[cfg(feature = "full_editor")]
mod events;
#[cfg(feature = "full_editor")]
mod file_manager;
#[cfg(feature = "full_editor")]
mod inspector;
#[cfg(feature = "full_editor")]
mod resources;
#[cfg(feature = "full_editor")]
mod systems;
#[cfg(feature = "full_editor")]
mod timeline;
#[cfg(feature = "full_editor")]
mod ui;

#[cfg(feature = "full_editor")]
use bevy::diagnostic::LogDiagnosticsPlugin;
#[cfg(feature = "full_editor")]
use bevy::prelude::*;
#[cfg(feature = "full_editor")]
use bevy::window::Window;
#[cfg(feature = "full_editor")]
use bevy_common_assets::ron::RonAssetPlugin;
#[cfg(feature = "full_editor")]
use bevy_inspector_egui::bevy_egui::{EguiPlugin, EguiPrimaryContextPass};
#[cfg(feature = "full_editor")]
use bevy_prototype_lyon::plugin::ShapePlugin;
#[cfg(feature = "full_editor")]
use carcinisation::{cutscene::data::CutsceneData, stage::data::StageData};
#[cfg(feature = "full_editor")]
use components::SceneData;
#[cfg(feature = "full_editor")]
use constants::assets_root;
#[cfg(feature = "full_editor")]
use file_manager::FileManagerPlugin;
#[cfg(feature = "full_editor")]
use inspector::InspectorPlugin;
#[cfg(feature = "full_editor")]
use resources::{CutsceneAssetHandle, StageAssetHandle, StageControlsUI};
#[cfg(feature = "full_editor")]
use systems::{
    animate_sprite, check_cutscene_data_loaded, check_stage_data_loaded,
    cutscene::update_cutscene_act_connections,
    input::{
        on_alt_mouse_motion, on_ctrl_mouse_motion, on_mouse_press, on_mouse_release, on_mouse_wheel,
    },
    maximize_window, on_scene_change, on_unload_scene, setup_camera,
};
#[cfg(feature = "full_editor")]
use ui::systems::update_ui;

#[cfg(feature = "full_editor")]
fn main() {
    let title: String = "SCENE EDITOR".to_string();

    App::new()
        .init_resource::<StageControlsUI>()
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    file_path: assets_root().to_string_lossy().to_string(),
                    meta_check: bevy::asset::AssetMetaCheck::Never,
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title,
                        resizable: true,
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins(InspectorPlugin)
        .add_plugins(LogDiagnosticsPlugin::default())
        .add_plugins(RonAssetPlugin::<CutsceneData>::new(&["cs.ron"]))
        .add_plugins(RonAssetPlugin::<StageData>::new(&["sg.ron"]))
        .add_plugins(EguiPlugin::default())
        .add_plugins(FileManagerPlugin)
        .add_plugins(ShapePlugin)
        .add_observer(on_unload_scene)
        .add_systems(Startup, (maximize_window, setup_camera))
        .add_systems(
            PreUpdate,
            on_scene_change.run_if(resource_exists::<SceneData>),
        )
        .add_systems(Update, update_cutscene_act_connections)
        .add_systems(
            Update,
            (check_cutscene_data_loaded.run_if(resource_exists::<CutsceneAssetHandle>),),
        )
        .add_systems(
            Update,
            (check_stage_data_loaded.run_if(resource_exists::<StageAssetHandle>),),
        )
        .add_systems(
            Update,
            (
                on_alt_mouse_motion,
                on_ctrl_mouse_motion,
                on_mouse_press,
                on_mouse_release,
                on_mouse_wheel,
            ),
        )
        .add_systems(EguiPrimaryContextPass, update_ui)
        .add_systems(Update, animate_sprite)
        .run();
}

#[cfg(not(feature = "full_editor"))]
fn main() {
    eprintln!(
        "tools/editor is currently disabled in this workspace build. \
        Re-run with `cargo run -p editor --features full_editor` to enable the full editor."
    );
}
