use bevy::prelude::*;
use carcinisation::{stage::data::StageData, CutsceneData};

/// Sprite atlas animation indices for cycling frames.
#[derive(Component)]
pub struct AnimationIndices {
    pub first: usize,
    pub last: usize,
}

/// Timer driving sprite atlas animation.
#[derive(Component, Deref, DerefMut)]
pub struct AnimationTimer(pub Timer);

/// Entity marker for cutscene act nodes.
#[derive(Component, Debug, Reflect)]
pub struct CutsceneActNode {
    pub act_index: usize,
}

/// Cutscene image sprite marker.
#[derive(Component, Debug, Reflect)]
pub struct CutsceneImage;

/// Stores the connection endpoints between cutscene acts.
#[derive(Component, Debug, Reflect)]
pub struct CutsceneActConnection {
    pub origin: Entity,
    pub target: Entity,
}

/// Marker for cutscene image labels.
#[derive(Component, Debug, Reflect)]
pub struct CutsceneImageLabel;

/// Marks entities draggable via editor input.
#[derive(Component, Debug, Reflect)]
pub struct Draggable;

/// Marker for the editor 2D camera.
#[derive(Component, Debug, Reflect)]
pub struct EditorCamera;

/// Marks letterbox labels in the cutscene view.
#[derive(Component, Debug, Reflect)]
pub struct LetterboxLabel;

/// Marks the currently selected editor entity.
#[derive(Component, Debug, Reflect)]
pub struct SelectedItem;

/// Marker for the selection outline entity.
#[derive(Component, Debug, Reflect)]
pub struct SelectionOutline;

/// Loaded scene data (stage or cutscene) for inspector/editor systems.
#[derive(Clone, Debug, Reflect, Resource)]
pub enum SceneData {
    Cutscene(CutsceneData),
    Stage(StageData),
}

/// Marker for entities spawned from the active scene.
#[derive(Component, Debug, Reflect)]
pub struct SceneItem;

/// Maps an editor entity to a stage spawn entry for in-place edits.
#[derive(Component, Copy, Clone, Debug, Reflect)]
pub enum StageSpawnRef {
    Static {
        index: usize,
    },
    Step {
        step_index: usize,
        spawn_index: usize,
        step_origin: Vec2,
    },
}

/// Current scene file path (absolute) for persistence and UI.
#[derive(Component, Debug, Default, Reflect, Resource)]
pub struct ScenePath(pub String);

/// Marker for stage spawn label entities.
#[derive(Component, Debug, Default, Reflect, Resource)]
pub struct StageSpawnLabel;
