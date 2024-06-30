use bevy::prelude::*;
use carcinisation::CutsceneData;

#[derive(Component, Debug, Reflect)]
pub struct CutsceneActNode {
    pub act_index: usize,
}

#[derive(Component, Debug, Reflect)]
pub struct CutsceneImage;

#[derive(Component, Debug, Reflect)]
pub struct CutsceneActConnection {
    pub origin: Entity,
    pub target: Entity,
}

#[derive(Component, Debug, Reflect)]
pub struct CutsceneImageLabel;

#[derive(Component, Debug, Reflect)]
pub struct Draggable;

#[derive(Component, Debug, Reflect)]
pub struct LetterboxLabel;

#[derive(Component, Debug, Reflect)]
pub struct SelectedItem;

#[derive(Clone, Debug, Reflect, Resource)]
pub enum LoadedScene {
    Cutscene(CutsceneData),
}

#[derive(Component, Debug, Reflect)]
pub struct SceneItem;
