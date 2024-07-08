pub mod components;
mod constants;
pub mod events;
mod systems;

use bevy::prelude::*;
use events::WriteRecentFilePathEvent;
use systems::*;

use crate::components::{SceneData, ScenePath};

pub struct FileManagerPlugin;

impl Plugin for FileManagerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ScenePath>()
            .add_event::<WriteRecentFilePathEvent>()
            .add_systems(Startup, (setup_ui, load_recent_file))
            .add_systems(
                Update,
                on_save_button_pressed.run_if(resource_exists::<SceneData>),
            )
            .add_systems(
                Update,
                (
                    on_button_interaction,
                    on_select_file_button_pressed,
                    poll_selected_file,
                    on_create_recent_file,
                ),
            );
    }
}
