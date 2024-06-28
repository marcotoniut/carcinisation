mod systems;

use crate::{components::LoadedScene, resources::CutsceneAssetHandle};

use self::systems::inspector_ui;
use bevy::prelude::*;
use bevy_inspector_egui::quick::ResourceInspectorPlugin;

pub struct InspectorPlugin;

impl Plugin for InspectorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(bevy_inspector_egui::DefaultInspectorConfigPlugin)
            .add_plugins(ResourceInspectorPlugin::<LoadedScene>::default())
            .add_systems(Update, inspector_ui);
    }
}
