use bevy::prelude::*;

#[derive(Component)]
pub struct Cinematic;

#[derive(Component)]
pub struct CutsceneEntity;

#[derive(Component)]
pub struct CutsceneGraphic;

#[derive(Component)]
pub struct LetterboxRow {
    pub row: u32,
}

pub const LETTERBOX_UPDATE_TIME: f32 = 0.015;

pub const LETTERBOX_HEIGHT: u32 = 30;
