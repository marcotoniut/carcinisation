use super::data::CutsceneData;
use bevy::prelude::*;
use std::sync::Arc;

#[derive(Event)]
pub struct CutsceneStartupTrigger {
    pub data: Arc<CutsceneData>,
}

#[derive(Clone, Event)]
pub struct CutsceneShutdownTrigger;
