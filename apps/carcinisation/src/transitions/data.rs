use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::globals::SCREEN_RESOLUTION;

#[derive(Clone, Debug, Deserialize, Reflect, Serialize)]
pub enum TransitionVenetianDataState {
    Opening,
    Closing,
}

#[derive(Clone, Debug, Deserialize, Reflect, Serialize)]
pub enum TransitionRequest {
    Venetian,
}

#[derive(Clone, Debug, Resource, Deserialize, Reflect, Serialize)]
pub struct TransitionVenetianData {
    pub state: TransitionVenetianDataState,
    #[serde(skip)]
    #[reflect(ignore)]
    pub buffer_rows: u32,
}

impl TransitionVenetianData {
    pub fn new(state: TransitionVenetianDataState) -> Self {
        let buffer_rows = (0.35 * SCREEN_RESOLUTION.y as f32) as u32;
        Self { state, buffer_rows }
    }
}
