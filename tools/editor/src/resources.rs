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
