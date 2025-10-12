//! Core cutscene markers used for spawned entities.

use bevy::prelude::*;

#[derive(Component)]
/// Root entity for the active cutscene scene graph.
pub struct CutsceneEntity;

#[derive(Component)]
/// Marks entities that should only run during cinematic playback.
pub struct Cinematic;

#[derive(Component)]
/// Marks spawned graphics belonging to the cutscene (cleanup helper).
pub struct CutsceneGraphic;
