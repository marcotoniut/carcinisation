use crate::{cutscene::data::CutsceneData, stage::data::StageData};
use bevy::prelude::*;
use derive_new::new;
use std::sync::Arc;

#[derive(new, Component, Clone, Debug)]
pub struct CinematicGameStep {
    pub data: Arc<CutsceneData>,
    #[new(value = "true")]
    pub is_checkpoint: bool,
}

impl CinematicGameStep {
    pub fn is_checkpoint(mut self, is_checkpoint: bool) -> Self {
        self.is_checkpoint = is_checkpoint;
        self
    }
}

#[derive(Component, Clone, Debug)]
pub struct CreditsGameStep;

#[derive(Component, Clone, Debug)]
pub struct TransitionGameStep {
    // TODO
    // pub transition: bool,
}

#[derive(new, Component, Clone, Debug)]
pub struct StageGameStep {
    pub data: Arc<StageData>,
}
