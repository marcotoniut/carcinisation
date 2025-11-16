//! Cutscene timing and progression resources shared by systems.

use bevy::prelude::*;

#[derive(Resource, Default, Debug, Clone, Copy)]
/// Marker for cutscene-local time domain.
pub struct CutsceneTimeDomain;

#[derive(Resource, Default, Clone, Copy)]
/// Index into the current cutscene act list.
pub struct CutsceneProgress {
    pub index: usize,
}
