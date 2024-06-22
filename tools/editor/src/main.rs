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
use carcinisation::cutscene::data::CutsceneData;
use constants::ASSETS_PATH;
use events::{CutsceneLoadedEvent, CutsceneUnloadedEvent};
use file_manager::FileManagerPlugin;
use inspector::InspectorPlugin;
use resources::CutsceneAssetHandle;
use systems::{check_cutscene_data_loaded, setup_camera};

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
        .add_event::<CutsceneLoadedEvent>()
        .add_event::<CutsceneUnloadedEvent>()
        .add_systems(Startup, setup_camera)
        .add_systems(Update, (display_cutscene_images,))
        .add_systems(
            Update,
            (check_cutscene_data_loaded.run_if(resource_exists::<CutsceneAssetHandle>),),
        )
        .run();
}

fn display_cutscene_images(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut event_reader: EventReader<CutsceneLoadedEvent>,
    mut camera_query: Query<&mut Transform, With<Camera>>,
) {
    let act_offset = 200.0;

    for e in event_reader.read() {
        let mut camera_transform = camera_query.single_mut();
        camera_transform.translation.x = act_offset * e.data.steps.len() as f32 / 2.0;

        for (act_index, act) in e.data.steps.iter().enumerate() {
            if let Some(spawn_images) = &act.spawn_images_o {
                for (image_index, image_spawn) in spawn_images.spawns.iter().enumerate() {
                    let transform = Transform {
                        translation: Vec3::new(
                            act_offset * act_index as f32,
                            180.0 * image_index as f32,
                            0.0,
                        ),
                        ..Default::default()
                    };

                    commands.spawn((
                        Name::new(format!(
                            "Act {} : Image {}",
                            act_index.to_string(),
                            image_index.to_string()
                        )),
                        SpriteBundle {
                            texture: asset_server.load(&image_spawn.image_path),
                            transform,
                            ..Default::default()
                        },
                    ));
                }
            }
        }
    }
}
