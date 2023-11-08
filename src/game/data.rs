use super::components::steps::*;

pub const STARTING_LIVES: u8 = 3;
pub const DEATH_SCORE_PENALTY: i32 = 150;

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
