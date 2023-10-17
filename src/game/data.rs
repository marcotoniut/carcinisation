use std::sync::Arc;

use bevy::prelude::*;

use crate::stage::data::StageData;

pub const STARTING_LIVES: u8 = 3;
pub const DEATH_SCORE_PENALTY: i32 = 150;

#[derive(Component, Clone, Debug)]
pub struct CinematicGameStep {
    // TODO
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
    pub stage_data: Arc<StageData>,
}

#[derive(Clone, Debug)]
pub enum GameStep {
    Cinematic(CinematicGameStep),
    Credits(CreditsGameStep),
    Transition(TransitionGameStep),
    Stage(StageGameStep),
}

#[derive(Clone, Debug, Resource)]
pub struct GameData {
    pub name: String,
    pub steps: Vec<GameStep>,
}
