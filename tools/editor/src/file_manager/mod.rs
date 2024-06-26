pub mod components;
mod constants;
mod systems;

use bevy::prelude::*;
use systems::*;

pub struct FileManagerPlugin;

impl Plugin for FileManagerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (setup_ui, load_recent_file))
            .add_systems(
                Update,
                (
                    on_button_interaction,
                    on_select_file,
                    on_save,
                    poll_selected_file,
                ),
            );
    }
}
