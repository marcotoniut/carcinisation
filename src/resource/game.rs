use crate::game::data::{GameData, GameStep, StageGameStep};
use crate::resource::asteroid::STAGE_ASTEROID_DATA;
use crate::resource::park::STAGE_PARK_DATA;
use crate::resource::spaceship::STAGE_SPACESHIP_DATA;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref GAME_DATA: GameData = GameData {
        name: "Main story".to_string(),
        steps: make_steps(),
    };
}

pub fn make_steps() -> Vec<GameStep> {
    vec![
        GameStep::Stage(StageGameStep {
            stage_data: STAGE_PARK_DATA.clone(),
        }),
        GameStep::Stage(StageGameStep {
            stage_data: STAGE_SPACESHIP_DATA.clone(),
        }),
        GameStep::Stage(StageGameStep {
            stage_data: STAGE_ASTEROID_DATA.clone(),
        }),
    ]
}
