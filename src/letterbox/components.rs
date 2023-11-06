use bevy::prelude::*;

#[derive(Component)]
pub struct LetterboxEntity;

#[derive(Component)]
pub struct LetterboxBottom;

#[derive(Component)]
pub struct LetterboxTop;

pub const LETTERBOX_UPDATE_TIME: f32 = 0.015;

pub const LETTERBOX_HEIGHT: u32 = 30;
