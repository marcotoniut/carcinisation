use carcinisation::cutscene::data::CutsceneData;

use bevy::prelude::*;

#[derive(Debug, Resource, Reflect)]
pub struct CutsceneAssetHandle {
    pub handle: Handle<CutsceneData>,
    pub path: String,
}
