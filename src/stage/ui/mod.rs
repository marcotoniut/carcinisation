pub mod cleared_screen;
pub mod components;
pub mod game_over_screen;
pub mod hud;
pub mod pause_menu;
pub mod systems;

use bevy::prelude::*;

use self::{
    hud::HudPlugin,
    systems::{
        state::{on_active, on_inactive},
        update_score_text,
    },
};

pub struct StageUiPlugin;

impl Plugin for StageUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<StageUiPluginUpdateState>()
            .add_systems(OnEnter(StageUiPluginUpdateState::Active), on_active)
            .add_systems(OnEnter(StageUiPluginUpdateState::Inactive), on_inactive)
            .add_plugins(HudPlugin)
            .add_systems(
                Update,
                update_score_text.run_if(in_state(StageUiPluginUpdateState::Active)),
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
