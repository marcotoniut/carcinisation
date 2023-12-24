pub mod cleared_screen;
pub mod components;
pub mod game_over_screen;
pub mod hud;
pub mod pause_menu;
pub mod systems;

use self::{
    cleared_screen::cleared_screen_plugin,
    game_over_screen::game_over_screen_plugin,
    hud::HudPlugin,
    systems::{
        state::{on_active, on_inactive},
        update_score_text,
    },
};
use bevy::prelude::*;
use seldom_fn_plugin::FnPluginExt;

pub struct StageUiPlugin;

impl Plugin for StageUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<StageUiPluginUpdateState>()
            .add_systems(OnEnter(StageUiPluginUpdateState::Active), on_active)
            .add_systems(OnEnter(StageUiPluginUpdateState::Inactive), on_inactive)
            .add_plugins(HudPlugin)
            .fn_plugin(cleared_screen_plugin)
            .fn_plugin(game_over_screen_plugin)
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
