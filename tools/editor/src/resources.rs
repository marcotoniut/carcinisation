use std::time::Duration;

use carcinisation::{cutscene::data::CutsceneData, stage::data::StageData};

use bevy::prelude::*;

#[derive(Debug, Resource, Reflect)]
pub struct CutsceneAssetHandle {
    pub handle: Handle<CutsceneData>,
    pub path: String,
}

#[derive(Debug, Resource, Reflect)]
pub struct StageAssetHandle {
    pub handle: Handle<StageData>,
    pub path: String,
}

#[derive(Debug, Resource, Reflect)]
pub struct StageElapsedUI(pub Duration);

impl Default for StageElapsedUI {
    fn default() -> Self {
        StageElapsedUI(Duration::from_secs(900))
    }
}
