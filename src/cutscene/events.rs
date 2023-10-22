use super::data::CutsceneData;
use bevy::prelude::*;
use std::sync::Arc;

#[derive(Event)]
pub struct CutsceneStartupEvent {
    pub data: Arc<CutsceneData>,
}

#[derive(Event)]
pub struct CutsceneShutdownEvent;
