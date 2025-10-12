//! Component types representing queued game steps.

use crate::{cutscene::data::CutsceneData, stage::data::StageData};
use bevy::prelude::*;
use derive_new::new;
use std::sync::Arc;

#[derive(new, Component, Clone, Debug)]
/// A cutscene step with fully loaded data ready to play.
pub struct CutsceneGameStep {
    pub data: Arc<CutsceneData>,
    #[new(value = "true")]
    pub is_checkpoint: bool,
}

#[derive(new, Component, Clone, Debug)]
/// A cutscene step referencing serialized data to stream in.
pub struct CinematicAssetGameStep {
    // pub data: Arc<CutsceneData>,
    pub src: String,
    #[new(value = "true")]
    pub is_checkpoint: bool,
}

#[derive(Component, Clone, Debug)]
/// Placeholder for credits sequence.
pub struct CreditsGameStep;

#[derive(Component, Clone, Debug)]
/// Placeholder for transitions between stages/cutscenes.
pub struct TransitionGameStep {
    // TODO
    // pub transition: bool,
}

#[derive(new, Component, Clone, Debug)]
/// Stage step with fully loaded stage data.
pub struct StageGameStep {
    pub data: Arc<StageData>,
}

#[derive(Component, Clone, Debug)]
/// Stage step referencing a serialized stage asset path.
pub struct StageAssetGameStep(pub String);
