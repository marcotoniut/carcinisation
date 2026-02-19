//! Defines the default sequence of game steps (cinematics and stages).

use crate::game::components::steps::{CinematicAssetGameStep, StageAssetGameStep};
use crate::game::data::GameStep;
use crate::game::resources::GameData;
use assert_assets_path::assert_assets_path;

/// Main campaign definition exposed to menus/startup.
pub static GAME_DATA: std::sync::LazyLock<GameData> = std::sync::LazyLock::new(|| GameData {
    name: "Main story".to_string(),
    steps: make_steps(),
});

/// Builds the ordered list of game steps for the campaign.
pub fn make_steps() -> Vec<GameStep> {
    vec![
        CinematicAssetGameStep {
            src: assert_assets_path!("cinematics/intro/data.cs.ron").to_string(),
            is_checkpoint: true,
        }
        .into(),
        // StageAssetGameStep(assert_assets_path!("stages/tester.sg.ron").to_string()).into(),
        StageAssetGameStep(assert_assets_path!("stages/park.sg.ron").to_string()).into(),
        // StageAssetGameStep(assert_assets_path!("stages/spaceship.sg.ron").to_string()).into(),
        // StageAssetGameStep(assert_assets_path!("stages/asteroid.sg.ron").to_string()).into(),
    ]
}
