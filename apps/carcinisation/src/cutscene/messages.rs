//! Cutscene lifecycle triggers.

use super::data::CutsceneData;
use bevy::prelude::*;
use std::sync::Arc;

#[derive(Event, Message)]
/// Fired to start a cutscene with the supplied data.
pub struct CutsceneStartupEvent {
    pub data: Arc<CutsceneData>,
}

#[derive(Clone, Event, Message)]
/// Fired when a cutscene finishes or is skipped.
pub struct CutsceneShutdownEvent;
