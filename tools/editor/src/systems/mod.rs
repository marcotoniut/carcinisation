pub mod cutscene;
pub mod input;

use bevy::asset::LoadState;
use bevy::window::{PrimaryWindow, WindowCloseRequested};
use bevy::{
    asset::{AssetServer, Assets},
    prelude::*,
};
use carcinisation::{CutsceneData, stage::data::StageData};

use crate::components::{
    AnimationIndices, AnimationTimer, EditorCamera, PathOverlay, PlacementGhost,
};
use crate::file_manager::events::WriteRecentFilePathEvent;
use crate::resources::{EditorState, StageControlsUI};
use crate::systems::input::{CoordinateOverlay, DragState};
use crate::{
    builders::{
        cutscene::spawn_cutscene,
        stage::{spawn_path, spawn_stage},
    },
    components::{SceneData, SceneItem, ScenePath},
    events::UnloadSceneTrigger,
    resources::{CutsceneAssetHandle, StageAssetHandle, ThumbnailCache},
};

/// @system Spawns the editor camera centered on the primary window.
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

/// @system Maximizes the primary window on startup.
pub fn maximize_window(mut window_query: Query<&mut Window, With<PrimaryWindow>>) {
    if let Ok(mut window) = window_query.single_mut() {
        window.set_maximized(true);
    }
}

/// Shared logic for processing an asset load state and installing the resulting scene.
fn handle_asset_load<H: Resource>(
    commands: &mut Commands,
    scene_path: &mut ScenePath,
    state: LoadState,
    path: &str,
    label: &str,
    data: Option<SceneData>,
) {
    match state {
        LoadState::Loaded => {
            if let Some(scene) = data {
                *scene_path = ScenePath(path.to_string());
                info!("{label} data loaded");
                commands.insert_resource(crate::resources::SavedSceneSnapshot::capture(&scene));
                commands.remove_resource::<H>();
                commands.insert_resource(scene);
                commands.trigger(WriteRecentFilePathEvent);
            } else {
                warn!("{label} data error");
            }
        }
        LoadState::Loading => {
            info!("{label} data is still loading...");
        }
        LoadState::NotLoaded => {
            warn!("{label} data is not loaded");
        }
        LoadState::Failed(e) => {
            commands.remove_resource::<H>();
            error!("{label} data failed to load: {e}");
        }
    }
}

/// @system Loads cutscene data once the asset server finishes loading the selected file.
#[allow(clippy::needless_pass_by_value)]
pub fn check_cutscene_data_loaded(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    cutscene_asset_handle: Res<CutsceneAssetHandle>,
    cutscene_data_assets: Res<Assets<CutsceneData>>,
    mut scene_path: ResMut<ScenePath>,
) {
    if let Some(state) = asset_server.get_load_state(cutscene_asset_handle.handle.id()) {
        let data = cutscene_data_assets
            .get(&cutscene_asset_handle.handle)
            .map(|d| SceneData::Cutscene(d.clone()));
        handle_asset_load::<CutsceneAssetHandle>(
            &mut commands,
            &mut scene_path,
            state,
            &cutscene_asset_handle.path,
            "Cutscene",
            data,
        );
    }
}

/// @system Loads stage data once the asset server finishes loading the selected file.
#[allow(clippy::needless_pass_by_value)]
pub fn check_stage_data_loaded(
    asset_server: Res<AssetServer>,
    stage_asset_handle: Res<StageAssetHandle>,
    stage_data_assets: Res<Assets<StageData>>,
    mut commands: Commands,
    mut scene_path: ResMut<ScenePath>,
) {
    if let Some(state) = asset_server.get_load_state(stage_asset_handle.handle.id()) {
        let data = stage_data_assets
            .get(&stage_asset_handle.handle)
            .map(|d| SceneData::Stage(d.clone()));
        handle_asset_load::<StageAssetHandle>(
            &mut commands,
            &mut scene_path,
            state,
            &stage_asset_handle.path,
            "Stage",
            data,
        );
    }
}

/// @system Rebuilds editor entities when the scene or visibility toggles change.
#[allow(clippy::needless_pass_by_value, clippy::too_many_arguments)]
pub fn on_scene_change(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    editor_state: Res<EditorState>,
    layer_shown_ui: Res<StageControlsUI>,
    loaded_scene: Res<SceneData>,
    scene_item_query: Query<Entity, With<SceneItem>>,
    overlay_query: Query<Entity, With<CoordinateOverlay>>,
    ghost_query: Query<Entity, With<PlacementGhost>>,
    mut image_assets: ResMut<Assets<Image>>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut thumbnail_cache: ResMut<ThumbnailCache>,
    drag_state: Res<DragState>,
    mut pending_rebuild: ResMut<crate::resources::PendingSceneRebuild>,
    depth_scale_config: Res<carcinisation::stage::depth_scale::DepthScaleConfig>,
) {
    // Skip full rebuild during an active path drag — the dragged handle entity must
    // stay alive. rebuild_path_during_drag handles the decorative geometry instead.
    if drag_state.active.as_ref().is_some_and(|d| d.kind.is_path()) {
        // Remember that a rebuild was requested so it happens when the drag ends.
        if loaded_scene.is_changed() || editor_state.is_changed() || layer_shown_ui.is_changed() {
            pending_rebuild.0 = true;
        }
        return;
    }

    if loaded_scene.is_changed()
        || editor_state.is_changed()
        || layer_shown_ui.is_changed()
        || pending_rebuild.0
    {
        pending_rebuild.0 = false;
        for entity in scene_item_query.iter() {
            commands.entity(entity).despawn();
        }
        for entity in overlay_query.iter() {
            commands.entity(entity).despawn();
        }
        for entity in ghost_query.iter() {
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
                    &editor_state,
                    &layer_shown_ui,
                    &data,
                    &mut image_assets,
                    &mut texture_atlas_layouts,
                    &mut thumbnail_cache,
                    &depth_scale_config,
                );
            }
        }
    }
}

/// @system Rebuilds the non-interactive path overlay entities (polyline, arrows, camera rect)
/// during an active path-node drag. The dragged node handle is kept alive — only the
/// decorative geometry is torn down and rebuilt from the latest StageData coordinates.
#[allow(clippy::needless_pass_by_value, clippy::type_complexity)]
pub fn rebuild_path_during_drag(
    mut commands: Commands,
    drag_state: Res<DragState>,
    scene_data: Option<Res<SceneData>>,
    stage_controls_ui: Res<StageControlsUI>,
    path_overlay_query: Query<
        Entity,
        (
            With<PathOverlay>,
            Without<crate::components::TweenPathNode>,
            Without<crate::components::StartCoordinatesNode>,
        ),
    >,
) {
    let Some(info) = &drag_state.active else {
        return;
    };
    if !info.kind.is_path() {
        return;
    }
    let Some(SceneData::Stage(stage_data)) = scene_data.as_deref() else {
        return;
    };

    // Despawn non-node path overlay entities (polyline, arrows, camera rect).
    // Node handles are excluded by the Without<TweenPathNode> filter.
    for entity in path_overlay_query.iter() {
        commands.entity(entity).despawn();
    }

    // Rebuild just the decorative path geometry from current data.
    spawn_path(&mut commands, stage_data, &stage_controls_ui, true);
}

/// @system Clears the current scene entities and resets the scene path.
pub fn on_unload_scene(
    _trigger: On<UnloadSceneTrigger>,
    mut commands: Commands,
    scene_item_query: Query<Entity, With<SceneItem>>,
    mut scene_path: ResMut<ScenePath>,
) {
    *scene_path = ScenePath(String::new());
    for entity in scene_item_query.iter() {
        commands.entity(entity).despawn();
    }
}

/// @system Advances sprite atlas frames when their animation timer ticks.
#[allow(clippy::needless_pass_by_value)]
pub fn animate_sprite(
    time: Res<Time>,
    mut query: Query<(&AnimationIndices, &mut AnimationTimer, &mut Sprite)>,
) {
    for (indices, mut timer, mut sprite) in &mut query {
        timer.tick(time.delta());
        if timer.just_finished()
            && let Some(atlas) = sprite.texture_atlas.as_mut()
        {
            atlas.index = if atlas.index == indices.last {
                indices.first
            } else {
                atlas.index + 1
            };
        }
    }
}

/// @system Intercepts window close requests. If there are unsaved changes, shows a
/// confirmation dialog instead of exiting immediately.
pub fn exit_on_window_close_request(
    mut close_requests: MessageReader<WindowCloseRequested>,
    mut exit: MessageWriter<AppExit>,
    snapshot: Res<crate::resources::SavedSceneSnapshot>,
    scene_data: Option<Res<SceneData>>,
    mut confirm: ResMut<crate::resources::CloseConfirmation>,
    should_exit: Res<crate::resources::ShouldExit>,
) {
    if should_exit.0 {
        exit.write(AppExit::Success);
        return;
    }
    if close_requests.read().next().is_some() {
        let has_unsaved = scene_data
            .as_ref()
            .is_some_and(|sd| snapshot.has_unsaved_changes(sd));
        if has_unsaved {
            confirm.0 = true;
        } else {
            exit.write(AppExit::Success);
        }
    }
}
