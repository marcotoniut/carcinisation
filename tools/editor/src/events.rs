use bevy::prelude::*;
use carcinisation::cutscene::data::CutsceneData;
use std::sync::Arc;

#[derive(Event)]
pub struct CutsceneLoadedEvent {
    pub data: Arc<CutsceneData>,
}

#[derive(Event)]
pub struct CutsceneUnloadedEvent;
