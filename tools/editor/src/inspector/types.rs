use bevy::prelude::*;

use crate::{
    components::*,
    resources::{CutsceneAssetHandle, StageAssetHandle},
};

pub fn register_types(app: &mut App) {
    app.register_type::<CutsceneActNode>()
        .register_type::<CutsceneImage>()
        .register_type::<CutsceneActConnection>()
        .register_type::<CutsceneAssetHandle>()
        .register_type::<CutsceneImageLabel>()
        .register_type::<Draggable>()
        .register_type::<LetterboxLabel>()
        .register_type::<SelectedItem>()
        .register_type::<StageAssetHandle>();
}
