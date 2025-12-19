pub mod cutscene;
pub mod input;

use bevy::asset::LoadState;
use bevy::window::PrimaryWindow;
use bevy::{
    asset::{AssetServer, Assets},
    prelude::*,
};
use carcinisation::{stage::data::StageData, CutsceneData};

use crate::components::{AnimationIndices, AnimationTimer, EditorCamera};
use crate::file_manager::events::WriteRecentFilePathEvent;
use crate::resources::StageControlsUI;
use crate::{
    builders::{cutscene::spawn_cutscene, stage::spawn_stage},
    components::{SceneData, SceneItem, ScenePath},
    events::UnloadSceneTrigger,
    resources::{CutsceneAssetHandle, StageAssetHandle},
};

pub fn setup_camera(mut commands: Commands, window_query: Query<&Window, With<PrimaryWindow>>) {
    let Ok(window) = window_query.single() else {
        return;
    };

    let camera_translation = Vec3::new(
        window.width() / 2.0 - 25.0,
        window.height() / 2.0 - 150.0,
        0.0,
    );

    commands.spawn((
        Camera2d,
        EditorCamera,
        Transform::from_translation(camera_translation),
    ));
}

pub fn maximize_window(mut window_query: Query<&mut Window, With<PrimaryWindow>>) {
    if let Ok(mut window) = window_query.single_mut() {
        window.set_maximized(true);
    }
}

pub fn check_cutscene_data_loaded(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    cutscene_asset_handle: Res<CutsceneAssetHandle>,
    cutscene_data_assets: Res<Assets<CutsceneData>>,
    mut scene_path: ResMut<ScenePath>,
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
                    commands.trigger(WriteRecentFilePathEvent);
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
                    commands.trigger(WriteRecentFilePathEvent);
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
            commands.entity(entity).despawn();
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
    _trigger: On<UnloadSceneTrigger>,
    mut commands: Commands,
    scene_item_query: Query<Entity, With<SceneItem>>,
    mut scene_path: ResMut<ScenePath>,
) {
    for entity in scene_item_query.iter() {
        *scene_path = ScenePath("".to_string());
        commands.entity(entity).despawn();
    }
}

pub fn animate_sprite(
    time: Res<Time>,
    mut query: Query<(&AnimationIndices, &mut AnimationTimer, &mut Sprite)>,
) {
    for (indices, mut timer, mut sprite) in &mut query {
        timer.tick(time.delta());
        if timer.just_finished() {
            if let Some(atlas) = sprite.texture_atlas.as_mut() {
                atlas.index = if atlas.index == indices.last {
                    indices.first
                } else {
                    atlas.index + 1
                };
            }
        }
    }
}
