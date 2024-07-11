pub mod cutscene;
pub mod input;

use bevy::asset::LoadState;
use bevy::window::PrimaryWindow;
use bevy::{
    asset::{AssetServer, Assets},
    prelude::*,
};
use carcinisation::{stage::data::StageData, CutsceneData};

use crate::components::{AnimationIndices, AnimationTimer};
// TODO should this be it
use crate::file_manager::events::WriteRecentFilePathEvent;
use crate::resources::StageControlsUI;
use crate::{
    builders::{cutscene::spawn_cutscene, stage::spawn_stage},
    components::{SceneData, SceneItem, ScenePath},
    events::UnloadSceneEvent,
    resources::{CutsceneAssetHandle, StageAssetHandle},
};

pub fn setup_camera(mut commands: Commands, window_query: Query<&Window, With<PrimaryWindow>>) {
    let Ok(window) = window_query.get_single() else {
        return;
    };

    let camera_translation = Vec3::new(
        window.width() / 2.0 - 25.0,
        window.height() / 2.0 - 150.0,
        0.0,
    );

    commands.spawn(Camera2dBundle {
        transform: Transform::from_translation(camera_translation),
        ..Default::default()
    });
}

pub fn check_cutscene_data_loaded(
    asset_server: Res<AssetServer>,
    cutscene_asset_handle: Res<CutsceneAssetHandle>,
    cutscene_data_assets: Res<Assets<CutsceneData>>,
    mut commands: Commands,
    mut scene_path: ResMut<ScenePath>,
    mut write_recent_file_path_event_writer: EventWriter<WriteRecentFilePathEvent>,
) {
    if let Some(state) = asset_server.get_load_state(cutscene_asset_handle.handle.id()) {
        match state {
            LoadState::Loaded => {
                if let Some(data) = cutscene_data_assets.get(&cutscene_asset_handle.handle) {
                    *scene_path = ScenePath(cutscene_asset_handle.path.to_string());
                    println!("Cutscene data loaded: {:?}", data);
                    commands.remove_resource::<CutsceneAssetHandle>();
                    commands.insert_resource(SceneData::Cutscene(data.clone()));
                    commands.insert_resource(ScenePath(cutscene_asset_handle.path.clone()));
                    write_recent_file_path_event_writer.send(WriteRecentFilePathEvent);
                } else {
                    println!("Cutscene data error");
                }
            }
            LoadState::Loading => {
                println!("Cutscene data is still loading...");
            }
            LoadState::NotLoaded => {
                println!("Cutscene data is not loaded");
            }
            LoadState::Failed(e) => {
                commands.remove_resource::<CutsceneAssetHandle>();
                println!("Cutscene data failed to load: {}", e.to_string());
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
    mut write_recent_file_path_event_writer: EventWriter<WriteRecentFilePathEvent>,
) {
    if let Some(state) = asset_server.get_load_state(stage_asset_handle.handle.id()) {
        match state {
            LoadState::Loaded => {
                if let Some(data) = stage_data_assets.get(&stage_asset_handle.handle) {
                    *scene_path = ScenePath(stage_asset_handle.path.to_string());
                    println!("Stage data loaded: {:?}", data);
                    commands.remove_resource::<StageAssetHandle>();
                    commands.insert_resource(SceneData::Stage(data.clone()));
                    commands.insert_resource(ScenePath(stage_asset_handle.path.clone()));
                    write_recent_file_path_event_writer.send(WriteRecentFilePathEvent);
                } else {
                    println!("Stage data error");
                }
            }
            LoadState::Loading => {
                println!("Stage data is still loading...");
            }
            LoadState::NotLoaded => {
                println!("Stage data is not loaded");
            }
            LoadState::Failed(e) => {
                commands.remove_resource::<StageAssetHandle>();
                println!("Stage data failed to load {}", e.to_string());
            }
        }
    }
}

pub fn on_scene_change(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    layer_shown_ui: Res<StageControlsUI>,
    loaded_scene: Res<SceneData>,
    scene_item_query: Query<Entity, With<SceneItem>>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    if loaded_scene.is_changed() || layer_shown_ui.is_changed() {
        for entity in scene_item_query.iter() {
            commands.entity(entity).despawn_recursive();
        }
        match loaded_scene.clone() {
            SceneData::Cutscene(data) => {
                spawn_cutscene(&mut commands, &asset_server, &data);
            }
            SceneData::Stage(data) => {
                spawn_stage(
                    &mut commands,
                    &asset_server,
                    &layer_shown_ui,
                    &data,
                    &mut texture_atlas_layouts,
                );
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

pub fn animate_sprite(
    time: Res<Time>,
    mut query: Query<(&AnimationIndices, &mut AnimationTimer, &mut TextureAtlas)>,
) {
    for (indices, mut timer, mut atlas) in &mut query {
        timer.tick(time.delta());
        if timer.just_finished() {
            atlas.index = if atlas.index == indices.last {
                indices.first
            } else {
                atlas.index + 1
            };
        }
    }
}
