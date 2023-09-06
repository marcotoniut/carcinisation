use bevy::prelude::*;

#[derive(Component)]
pub struct Cutscene;

#[derive(Component)]
pub struct LetterboxRow {
    pub row: u32,
}

pub const LETTERBOX_UPDATE_TIME: f32 = 0.015;

pub const LETTERBOX_HEIGHT: u32 = 30;
