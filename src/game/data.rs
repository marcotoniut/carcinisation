//! Shared constants and enums describing game progression steps.

use super::components::steps::*;
use derive_more::From;

pub const STARTING_LIVES: u8 = 3;
pub const DEATH_SCORE_PENALTY: i32 = 150;

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
