mod systems;
mod types;

use self::systems::inspector_ui;
use bevy::prelude::*;

pub struct InspectorPlugin;

impl Plugin for InspectorPlugin {
    fn build(&self, app: &mut App) {
        carcinisation::debug::types::register_types(app);
        self::types::register_types(app);
        app.add_plugins(bevy_inspector_egui::DefaultInspectorConfigPlugin)
            .add_systems(Update, inspector_ui);
    }
}
