pub mod actions;
pub mod components;
mod constants;
pub mod events;
mod systems;

use bevy::prelude::*;
use events::WriteRecentFilePathEvent;
use systems::*;

use crate::components::ScenePath;

/// Handles recent-file persistence and scene file selection.
pub struct FileManagerPlugin;

impl Plugin for FileManagerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ScenePath>()
            .add_message::<WriteRecentFilePathEvent>()
            .add_observer(on_write_recent_file_path)
            .add_systems(Startup, load_recent_file)
            .add_systems(Update, poll_selected_file);
    }
}
