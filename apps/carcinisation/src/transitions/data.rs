use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "derive-ts")]
use ts_rs::TS;

use crate::globals::SCREEN_RESOLUTION;

#[cfg_attr(feature = "derive-ts", derive(TS))]
#[cfg_attr(feature = "derive-ts", ts(export))]
#[derive(Clone, Debug, Deserialize, Reflect, Serialize)]
pub enum TransitionVenetianDataState {
    Opening,
    Closing,
}

#[cfg_attr(feature = "derive-ts", derive(TS))]
#[cfg_attr(feature = "derive-ts", ts(export))]
#[derive(Clone, Debug, Deserialize, Reflect, Serialize)]
pub enum TransitionRequest {
    Venetian,
}

#[cfg_attr(feature = "derive-ts", derive(TS))]
#[cfg_attr(feature = "derive-ts", ts(export))]
#[derive(Clone, Debug, Resource, Deserialize, Reflect, Serialize)]
pub struct TransitionVenetianData {
    pub state: TransitionVenetianDataState,
    #[serde(skip)]
    #[reflect(ignore)]
    #[cfg_attr(feature = "derive-ts", ts(skip))]
    pub buffer_rows: u32,
}

impl TransitionVenetianData {
    pub fn new(state: TransitionVenetianDataState) -> Self {
        let buffer_rows = (0.35 * SCREEN_RESOLUTION.y as f32) as u32;
        Self { state, buffer_rows }
    }
}
