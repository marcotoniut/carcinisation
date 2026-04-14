use std::{collections::HashMap, time::Duration};

use carcinisation::{
    cutscene::data::CutsceneData,
    stage::{components::placement::Depth, data::StageData, enemy::entity::EnemyType},
};

use bevy::{prelude::*, sprite::Anchor};
use serde::{Deserialize, Serialize};

/// Editor-wide mode flags that affect scene presentation but not saved stage data.
#[derive(Clone, Debug, Default, Reflect, Resource)]
#[reflect(Resource)]
pub struct EditorState {
    pub depth_preview_enabled: bool,
}

/// Active cutscene asset handle and source path.
#[derive(Debug, Reflect, Resource)]
pub struct CutsceneAssetHandle {
    pub handle: Handle<CutsceneData>,
    pub path: String,
}

/// Active stage asset handle and source path.
#[derive(Debug, Reflect, Resource)]
pub struct StageAssetHandle {
    pub handle: Handle<StageData>,
    pub path: String,
}

/// Cached generated thumbnails used by editor scene previews.
///
/// Key: `(EnemyType, Depth, animation_tag)`. The tag disambiguates previews
/// for multi-state composed enemies.
#[derive(Debug, Default, Resource)]
pub struct ThumbnailCache {
    pub composed_enemies: HashMap<(EnemyType, Depth, String), CachedThumbnail>,
}

#[derive(Clone, Debug)]
pub struct CachedThumbnail {
    pub image: Handle<Image>,
    pub anchor: Anchor,
    pub fallback_scale: f32,
}

/// UI state for editor stage controls and layer visibility.
#[derive(Clone, Debug, Reflect, Resource, Deserialize, Serialize)]
#[reflect(Resource)]
#[serde(rename_all = "PascalCase")]
#[allow(clippy::struct_excessive_bools)]
pub struct StageControlsUI {
    pub elapsed_duration: Duration,

    pub elapsed_path: bool,
    pub show_all_spawns: bool,

    pub skybox: bool,
    pub background: bool,

    pub nine: bool,
    pub eight: bool,
    pub seven: bool,
    pub six: bool,
    pub five: bool,
    pub four: bool,
    pub three: bool,
    pub two: bool,
    pub one: bool,
    pub zero: bool,
}

impl Default for StageControlsUI {
    fn default() -> Self {
        StageControlsUI {
            elapsed_path: true,
            show_all_spawns: false,
            elapsed_duration: Duration::from_secs(999),
            skybox: true,
            background: true,
            nine: true,
            eight: true,
            seven: true,
            six: true,
            five: true,
            four: true,
            three: true,
            two: true,
            one: true,
            zero: true,
        }
    }
}

impl StageControlsUI {
    /// Whether the elapsed camera path overlay is visible.
    pub fn path_is_visible(&self) -> bool {
        self.elapsed_path
    }

    /// Whether the stage background is visible.
    pub fn background_is_visible(&self) -> bool {
        self.background
    }

    /// Whether the stage skybox is visible.
    pub fn skybox_is_visible(&self) -> bool {
        self.skybox
    }

    /// Whether entities at the requested depth should be rendered.
    pub fn depth_is_visible(&self, depth: Depth) -> bool {
        match depth {
            Depth::Nine => self.nine,
            Depth::Eight => self.eight,
            Depth::Seven => self.seven,
            Depth::Six => self.six,
            Depth::Five => self.five,
            Depth::Four => self.four,
            Depth::Three => self.three,
            Depth::Two => self.two,
            Depth::One => self.one,
            Depth::Zero => self.zero,
        }
    }
}

/// Holds a RON snapshot of the scene at last save/load.
/// Comparing the current SceneData serialization against this detects unsaved changes
/// without relying on change-detection flags.
#[derive(Resource, Default, Debug)]
pub struct SavedSceneSnapshot(pub Option<String>);

impl SavedSceneSnapshot {
    /// Captures the current SceneData as a RON string.
    pub fn capture(scene_data: &crate::components::SceneData) -> Self {
        let ron_str = match scene_data {
            crate::components::SceneData::Cutscene(data) => Self::to_ron(data),
            crate::components::SceneData::Stage(data) => Self::to_ron(data),
        };
        Self(Some(ron_str))
    }

    /// Returns true if the current scene differs from the saved snapshot.
    pub fn has_unsaved_changes(&self, current: &crate::components::SceneData) -> bool {
        let Some(ref saved) = self.0 else {
            return false;
        };
        let current_ron = match current {
            crate::components::SceneData::Cutscene(data) => Self::to_ron(data),
            crate::components::SceneData::Stage(data) => Self::to_ron(data),
        };
        *saved != current_ron
    }

    fn to_ron<T: serde::Serialize>(data: &T) -> String {
        let config = ron::ser::PrettyConfig::new()
            .struct_names(true)
            .extensions(ron::extensions::Extensions::all());
        ron::ser::to_string_pretty(data, config).unwrap_or_default()
    }
}

/// When true, the close-confirmation dialog is shown.
#[derive(Resource, Default, Debug)]
pub struct CloseConfirmation(pub bool);

/// Set to true to exit the app on the next frame.
#[derive(Resource, Default, Debug)]
pub struct ShouldExit(pub bool);

/// Set to true to force a full scene rebuild on the next frame.
/// Used to defer rebuilds that were skipped during an active path drag.
#[derive(Resource, Default, Debug)]
pub struct PendingSceneRebuild(pub bool);

/// Persistent UI state for the Scene inspector split layout.
#[derive(Resource, Debug)]
pub struct SceneInspectorLayout {
    pub selection_height: f32,
}

impl Default for SceneInspectorLayout {
    fn default() -> Self {
        Self {
            selection_height: 220.0,
        }
    }
}
