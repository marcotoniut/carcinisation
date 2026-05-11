//! Shared constants and enums describing game progression steps.

use super::components::steps::{
    CinematicAssetGameStep, CreditsGameStep, CutsceneGameStep, StageAssetGameStep, StageGameStep,
    TransitionGameStep,
};
use derive_more::From;

pub use carcinisation_base::game::{DEATH_SCORE_PENALTY, STARTING_LIVES};

#[derive(Clone, Debug, From)]
/// Union of all supported game steps (cinematics, stages, transitions).
pub enum GameStep {
    Credits(CreditsGameStep),
    Cutscene(CutsceneGameStep),
    CutsceneAsset(CinematicAssetGameStep),
    Stage(StageGameStep),
    StageAsset(StageAssetGameStep),
    Transition(TransitionGameStep),
}
