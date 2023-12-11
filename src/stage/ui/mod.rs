pub mod cleared_screen;
pub mod components;
pub mod game_over_screen;
pub mod hud;
pub mod pause_menu;
pub mod systems;

use self::{
    cleared_screen::{
        events::ClearScreenShutdownEvent, resources::ClearScreenInput,
        setup::init_clear_screen_input, systems::check_press_continue_input,
    },
    hud::HudPlugin,
    systems::{
        state::{on_active, on_inactive},
        update_score_text,
    },
};
use bevy::prelude::*;
use leafwing_input_manager::plugin::InputManagerPlugin;

pub struct StageUiPlugin;

impl Plugin for StageUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<StageUiPluginUpdateState>()
            .add_systems(OnEnter(StageUiPluginUpdateState::Active), on_active)
            .add_systems(OnEnter(StageUiPluginUpdateState::Inactive), on_inactive)
            .add_plugins(HudPlugin)
            // TODO move into its own plugin
            .add_event::<ClearScreenShutdownEvent>()
            .add_plugins(InputManagerPlugin::<ClearScreenInput>::default())
            .add_systems(Startup, (init_clear_screen_input))
            .add_systems(
                Update,
                update_score_text.run_if(in_state(StageUiPluginUpdateState::Active)),
            )
            .add_systems(
                PostUpdate,
                check_press_continue_input.run_if(in_state(StageUiPluginUpdateState::Active)),
            );
        // .add_plugins(PauseScreenPlugin);
    }
}

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
pub enum StageUiPluginUpdateState {
    #[default]
    Inactive,
    Active,
}
