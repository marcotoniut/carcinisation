pub mod cutscene;
pub mod input;

use bevy::asset::LoadState;
use bevy::{
    asset::{AssetServer, Assets},
    prelude::*,
};
use carcinisation::{stage::data::StageData, CutsceneData};

use crate::resources::StageElapsedUI;
use crate::{
    builders::{cutscene::spawn_cutscene, stage::spawn_stage},
    components::{SceneData, SceneItem, ScenePath},
    events::UnloadSceneEvent,
    resources::{CutsceneAssetHandle, StageAssetHandle},
};

pub fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

pub fn check_cutscene_data_loaded(
    asset_server: Res<AssetServer>,
    cutscene_asset_handle: Res<CutsceneAssetHandle>,
    cutscene_data_assets: Res<Assets<CutsceneData>>,
    mut commands: Commands,
    mut scene_path: ResMut<ScenePath>,
) {
    if let Some(state) = asset_server.get_load_state(cutscene_asset_handle.handle.clone()) {
        match state {
            LoadState::Loaded => {
                if let Some(data) = cutscene_data_assets.get(&cutscene_asset_handle.handle) {
                    *scene_path = ScenePath(cutscene_asset_handle.path.to_string());
                    println!("Cutscene data loaded: {:?}", data);
                    commands.remove_resource::<CutsceneAssetHandle>();
                    commands.insert_resource(SceneData::Cutscene(data.clone()));
                    commands.insert_resource(ScenePath(cutscene_asset_handle.path.clone()));
                } else {
                }
            }
            LoadState::Loading => {
                println!("Cutscene data is still loading...");
            }
            LoadState::NotLoaded => {
                println!("Cutscene data is not loaded");
            }
            LoadState::Failed => {
                commands.remove_resource::<CutsceneAssetHandle>();
                println!("Cutscene data failed to load");
            }
        }
    }
}

pub fn check_stage_data_loaded(
    asset_server: Res<AssetServer>,
    stage_asset_handle: Res<StageAssetHandle>,
    stage_data_assets: Res<Assets<StageData>>,
    mut commands: Commands,
    mut scene_path: ResMut<ScenePath>,
) {
    if let Some(state) = asset_server.get_load_state(stage_asset_handle.handle.clone()) {
        match state {
            LoadState::Loaded => {
                if let Some(data) = stage_data_assets.get(&stage_asset_handle.handle) {
                    *scene_path = ScenePath(stage_asset_handle.path.to_string());
                    println!("Stage data loaded: {:?}", data);
                    commands.remove_resource::<StageAssetHandle>();
                    commands.insert_resource(SceneData::Stage(data.clone()));
                    commands.insert_resource(ScenePath(stage_asset_handle.path.clone()));
                } else {
                }
            }
            LoadState::Loading => {
                println!("Stage data is still loading...");
            }
            LoadState::NotLoaded => {
                println!("Stage data is not loaded");
            }
            LoadState::Failed => {
                commands.remove_resource::<StageAssetHandle>();
                println!("Stage data failed to load");
            }
        }
    }
}

pub fn on_scene_change(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    loaded_scene: Res<SceneData>,
    stage_elapsed_ui: Res<StageElapsedUI>,
    scene_item_query: Query<Entity, With<SceneItem>>,
) {
    if loaded_scene.is_changed() || stage_elapsed_ui.is_changed() {
        for entity in scene_item_query.iter() {
            commands.entity(entity).despawn_recursive();
        }
        match loaded_scene.clone() {
            SceneData::Cutscene(data) => {
                spawn_cutscene(&mut commands, &asset_server, &data);
            }
            SceneData::Stage(data) => {
                spawn_stage(&mut commands, &asset_server, &stage_elapsed_ui, &data);
            }
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
