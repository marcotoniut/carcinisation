use crate::game::components::steps::*;
use crate::game::data::GameStep;
use crate::game::resources::GameData;
use assert_assets_path::assert_assets_path;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref GAME_DATA: GameData = GameData {
        name: "Main story".to_string(),
        steps: make_steps(),
    };
}

pub fn make_steps() -> Vec<GameStep> {
    vec![
        CinematicAssetGameStep {
            src: assert_assets_path!("cinematics/intro/data.cs.ron").to_string(),
            is_checkpoint: true,
        }
        .into(),
        StageAssetGameStep(assert_assets_path!("stages/debug.sg.ron").to_string()).into(),
        // StageAssetGameStep(assert_assets_path!("stages/park.sg.ron").to_string()).into(),
        // StageAssetGameStep(assert_assets_path!("stages/spaceship.sg.ron").to_string()).into(),
        // StageAssetGameStep(assert_assets_path!("stages/asteroid.sg.ron").to_string()).into(),
    ]
}
