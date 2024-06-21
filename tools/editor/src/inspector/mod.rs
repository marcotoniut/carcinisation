mod systems;

use self::systems::inspector_ui;
use bevy::prelude::*;

pub struct InspectorPlugin;

impl Plugin for InspectorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(bevy_inspector_egui::DefaultInspectorConfigPlugin)
            .add_systems(Update, inspector_ui);
    }
}
