use super::data::CinematicData;
use bevy::prelude::*;
use std::sync::Arc;

#[derive(Event)]
pub struct CinematicStartupEvent {
    pub data: Arc<CinematicData>,
}

#[derive(Event)]
pub struct CutsceneShutdownEvent;
