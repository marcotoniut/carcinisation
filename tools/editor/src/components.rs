use bevy::prelude::*;

#[derive(Component, Debug)]
pub struct CutsceneActNode;

#[derive(Component, Debug)]
pub struct CutsceneImage;

#[derive(Component, Debug)]
pub struct CutsceneActConnection {
    pub origin: Entity,
    pub target: Entity,
}

#[derive(Component, Debug)]
pub struct CutsceneImageLabel;

#[derive(Component, Debug)]
pub struct Draggable;

#[derive(Component, Debug)]
pub struct LetterboxLabel;

#[derive(Component, Debug)]
pub struct SelectedItem;
