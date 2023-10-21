use crate::{cutscene::data::CinematicData, stage::data::StageData};
use bevy::prelude::*;
use std::sync::Arc;

pub const STARTING_LIVES: u8 = 3;
pub const DEATH_SCORE_PENALTY: i32 = 150;

#[derive(Component, Clone, Debug)]
pub struct CinematicGameStep {
    pub data: Arc<CinematicData>,
    // pub cinematic: bool,
    // pub is_checkpoint: bool,
    // pub music_fade: bool,
    // pub music_track: Option<String>,
}

#[derive(Component, Clone, Debug)]
pub struct CreditsGameStep {}

#[derive(Component, Clone, Debug)]
pub struct TransitionGameStep {
    // TODO
    // pub transition: bool,
}

#[derive(Component, Clone, Debug)]
pub struct StageGameStep {
    pub data: Arc<StageData>,
}

#[derive(Clone, Debug)]
pub enum GameStep {
    Cinematic(CinematicGameStep),
    Credits(CreditsGameStep),
    Transition(TransitionGameStep),
    Stage(StageGameStep),
}

impl From<CinematicGameStep> for GameStep {
    fn from(step: CinematicGameStep) -> Self {
        GameStep::Cinematic(step)
    }
}

impl From<CreditsGameStep> for GameStep {
    fn from(step: CreditsGameStep) -> Self {
        GameStep::Credits(step)
    }
}

impl From<TransitionGameStep> for GameStep {
    fn from(step: TransitionGameStep) -> Self {
        GameStep::Transition(step)
    }
}

impl From<StageGameStep> for GameStep {
    fn from(step: StageGameStep) -> Self {
        GameStep::Stage(step)
    }
}

#[derive(Clone, Debug, Resource)]
pub struct GameData {
    pub name: String,
    pub steps: Vec<GameStep>,
}
