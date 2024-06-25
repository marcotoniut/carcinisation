mod components;
mod constants;
mod events;
mod file_manager;
mod inspector;
mod resources;
mod systems;
mod ui;

use bevy::diagnostic::LogDiagnosticsPlugin;
use bevy::prelude::*;
use bevy::window::Window;
use bevy_common_assets::ron::RonAssetPlugin;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_prototype_lyon::plugin::ShapePlugin;
use carcinisation::cutscene::data::CutsceneData;
use constants::ASSETS_PATH;
use events::{CutsceneLoadedEvent, CutsceneUnloadedEvent};
use file_manager::FileManagerPlugin;
use inspector::InspectorPlugin;
use resources::CutsceneAssetHandle;
use systems::{
    check_cutscene_data_loaded,
    cutscene::{display_cutscene_acts, update_cutscene_act_connections},
    input::{on_mouse_motion, on_mouse_press, on_mouse_release, on_mouse_wheel},
    setup_camera,
};

// #[derive(Resource)]
// pub struct SelectedFileText(Option<String>);

fn main() {
    let title: String = "SCENE EDITOR".to_string();

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    file_path: ASSETS_PATH.into(),
                    ..Default::default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title,
                        resizable: true,
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
        )
        .add_plugins(LogDiagnosticsPlugin::default())
        .add_plugins(RonAssetPlugin::<CutsceneData>::new(&["cs.ron"]))
        .add_plugins(EguiPlugin)
        .add_plugins(InspectorPlugin)
        .add_plugins(FileManagerPlugin)
        .add_plugins(ShapePlugin)
        .add_event::<CutsceneLoadedEvent>()
        .add_event::<CutsceneUnloadedEvent>()
        .add_systems(Startup, setup_camera)
        .add_systems(
            Update,
            (display_cutscene_acts, update_cutscene_act_connections),
        )
        .add_systems(
            Update,
            (check_cutscene_data_loaded.run_if(resource_exists::<CutsceneAssetHandle>),),
        )
        .add_systems(
            Update,
            (
                on_mouse_motion,
                on_mouse_press,
                on_mouse_release,
                on_mouse_wheel,
            ),
        )
        .run();
}
