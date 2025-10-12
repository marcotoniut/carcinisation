//! Cutscene lifecycle triggers.

use super::data::CutsceneData;
use bevy::prelude::*;
use std::sync::Arc;

#[derive(Event)]
/// Fired to start a cutscene with the supplied data.
pub struct CutsceneStartupTrigger {
    pub data: Arc<CutsceneData>,
}

#[derive(Clone, Event)]
/// Fired when a cutscene finishes or is skipped.
pub struct CutsceneShutdownTrigger;
