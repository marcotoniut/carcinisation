use bevy::prelude::*;

#[derive(Component)]
pub struct LetterboxEntity;

#[derive(Component)]
pub struct LetterboxBottom;

#[derive(Component)]
pub struct LetterboxTop;

pub const LETTERBOX_NORMAL_SPEED: f32 = 30.;
pub const LETTERBOX_INSTANT_SPEED: f32 = f32::MAX;

pub const LETTERBOX_HEIGHT: u32 = 30;
