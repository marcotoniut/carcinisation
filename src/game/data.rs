use super::components::steps::*;
use derive_more::From;

pub const STARTING_LIVES: u8 = 3;
pub const DEATH_SCORE_PENALTY: i32 = 150;

#[derive(Clone, Debug, From)]
pub enum GameStep {
    Credits(CreditsGameStep),
    Cutscene(CutsceneGameStep),
    CutsceneAsset(CinematicAssetGameStep),
    Stage(StageGameStep),
    StageAsset(StageAssetGameStep),
    Transition(TransitionGameStep),
}
