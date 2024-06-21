use carcinisation::cutscene::data::CutsceneData;

use bevy::prelude::*;

#[derive(Resource)]
pub struct CutsceneAssetHandle {
    pub handle: Handle<CutsceneData>,
}
