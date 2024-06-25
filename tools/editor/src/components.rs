use bevy::prelude::*;

#[derive(Component, Debug, Reflect)]
pub struct CutsceneActNode;

#[derive(Component, Debug, Reflect)]
pub struct CutsceneImage;

#[derive(Component, Debug, Reflect)]
pub struct CutsceneActConnection {
    pub origin: Entity,
    pub target: Entity,
}

#[derive(Component, Debug, Reflect)]
pub struct CutsceneActLabel;

#[derive(Component, Debug, Reflect)]
pub struct CutsceneImageLabel;

#[derive(Component, Debug, Reflect)]
pub struct Draggable;

#[derive(Component, Debug, Reflect)]
pub struct LetterboxLabel;

#[derive(Component, Debug, Reflect)]
pub struct SelectedItem;
