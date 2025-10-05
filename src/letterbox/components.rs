//! Components and constants for letterbox UI bars.

use bevy::prelude::*;

#[derive(Component)]
/// Marker for entities belonging to the letterbox system.
pub struct LetterboxEntity;

#[derive(Component)]
/// Marker for the bottom bar entity.
pub struct LetterboxBottom;

#[derive(Component)]
/// Marker for the top bar entity.
pub struct LetterboxTop;

pub const LETTERBOX_NORMAL_SPEED: f32 = 10.;
pub const LETTERBOX_INSTANT_SPEED: f32 = f32::MAX;

pub const LETTERBOX_HEIGHT: u32 = 30;
