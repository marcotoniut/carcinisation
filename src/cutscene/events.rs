use super::data::CinemachineData;
use bevy::prelude::*;
use std::sync::Arc;

#[derive(Event)]
pub struct CinematicStartupEvent {
    pub data: Arc<Vec<CinemachineData>>,
}
