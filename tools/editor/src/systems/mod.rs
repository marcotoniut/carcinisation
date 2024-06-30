pub mod cutscene;
pub mod input;

use bevy::{
    asset::{AssetServer, Assets},
    prelude::*,
};
use carcinisation::CutsceneData;

use crate::{
    builders::cutscene::spawn_cutscene,
    components::{SceneData, SceneItem, ScenePath},
    events::UnloadSceneEvent,
    resources::CutsceneAssetHandle,
};

pub fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

pub fn check_cutscene_data_loaded(
    cutscene_asset_handle: Res<CutsceneAssetHandle>,
    cutscene_data_assets: Res<Assets<CutsceneData>>,
    mut commands: Commands,
) {
    if let Some(cutscene_data) = cutscene_data_assets.get(&cutscene_asset_handle.handle) {
        println!("Cutscene data loaded: {:?}", cutscene_data);
        commands.remove_resource::<CutsceneAssetHandle>();
        commands.insert_resource(SceneData::Cutscene(cutscene_data.clone()));
        commands.insert_resource(ScenePath(cutscene_asset_handle.path.clone()));
    } else {
        // Asset is not yet loaded
        println!("Cutscene data is still loading...");
    }
}

pub fn on_loaded_scene(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    loaded_scene: Res<SceneData>,
    scene_item_query: Query<Entity, With<SceneItem>>,
    // mut camera_query: Query<&mut Transform, With<Camera>>,
) {
    if loaded_scene.is_changed() {
        for entity in scene_item_query.iter() {
            commands.entity(entity).despawn_recursive();
        }
        match loaded_scene.clone() {
            SceneData::Cutscene(data) => {
                spawn_cutscene(&mut commands, &asset_server, &data);
            }
            _ => {}
        }
    }
}

pub fn on_unload_scene(
    mut commands: Commands,
    scene_item_query: Query<Entity, With<SceneItem>>,
    mut scene_path: ResMut<ScenePath>,
    mut unload_scene_event_reader: EventReader<UnloadSceneEvent>,
) {
    for _ in unload_scene_event_reader.read() {
        for entity in scene_item_query.iter() {
            *scene_path = ScenePath("".to_string());
            commands.entity(entity).despawn_recursive();
        }
    }
}
