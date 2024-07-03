use crate::{cutscene::data::CutsceneData, stage::data::StageData};
use bevy::prelude::*;
use derive_new::new;
use std::sync::Arc;

#[derive(new, Component, Clone, Debug)]
pub struct CutsceneGameStep {
    pub data: Arc<CutsceneData>,
    #[new(value = "true")]
    pub is_checkpoint: bool,
}

#[derive(new, Component, Clone, Debug)]
pub struct CinematicAssetGameStep {
    // pub data: Arc<CutsceneData>,
    pub src: String,
    #[new(value = "true")]
    pub is_checkpoint: bool,
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

#[derive(Component, Clone, Debug)]
pub struct StageAssetGameStep(pub String);
