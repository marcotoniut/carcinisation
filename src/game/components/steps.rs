use crate::{cutscene::data::CutsceneData, stage::data::StageData};
use bevy::prelude::*;
use std::sync::Arc;

#[derive(Component, Clone, Debug)]
pub struct CinematicGameStep {
    pub data: Arc<CutsceneData>,
    pub is_checkpoint: bool,
}

impl CinematicGameStep {
    pub fn new(data: Arc<CutsceneData>) -> Self {
        Self {
            data,
            is_checkpoint: true,
        }
    }

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

#[derive(Component, Clone, Debug)]
pub struct StageGameStep {
    pub data: Arc<StageData>,
}

impl StageGameStep {
    pub fn new(data: Arc<StageData>) -> Self {
        Self { data }
    }
}
