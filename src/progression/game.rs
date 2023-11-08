use crate::game::components::steps::*;
use crate::game::data::GameStep;
use crate::game::resources::GameData;
use crate::progression::cinematics::intro::CINEMATIC_INTRO_DATA;
use crate::progression::stages::asteroid::STAGE_ASTEROID_DATA;
use crate::progression::stages::park::STAGE_PARK_DATA;
use crate::progression::stages::spaceship::STAGE_SPACESHIP_DATA;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref GAME_DATA: GameData = GameData {
        name: "Main story".to_string(),
        steps: make_steps(),
    };
}

pub fn make_steps() -> Vec<GameStep> {
    vec![
        CinematicGameStep::new(CINEMATIC_INTRO_DATA.clone()).into(),
        StageGameStep::new(STAGE_PARK_DATA.clone()).into(),
        StageGameStep::new(STAGE_SPACESHIP_DATA.clone()).into(),
        StageGameStep::new(STAGE_ASTEROID_DATA.clone()).into(),
    ]
}
