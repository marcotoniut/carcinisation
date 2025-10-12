//! Stage-specific UI overlays: HUD, cleared/death/game-over screens, and state gating.

pub mod cleared_screen;
pub mod components;
pub mod death_screen;
pub mod game_over_screen;
pub mod hud;
pub mod pause_menu;
mod systems;

use self::{
    cleared_screen::cleared_screen_plugin,
    death_screen::death_screen_plugin,
    game_over_screen::game_over_screen_plugin,
    hud::HudPlugin,
    systems::{
        state::{on_active, on_inactive},
        update_score_text,
    },
};
use bevy::prelude::*;

/// Registers all stage UI sub-plugins and manages their active state.
pub struct StageUiPlugin;

impl Plugin for StageUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(HudPlugin)
            .add_plugins((
                cleared_screen_plugin,
                death_screen_plugin,
                game_over_screen_plugin,
            ))
            .init_state::<StageUiPluginUpdateState>()
            .add_systems(OnEnter(StageUiPluginUpdateState::Active), on_active)
            .add_systems(OnEnter(StageUiPluginUpdateState::Inactive), on_inactive)
            .add_systems(
                Update,
                // HUD score updates when UI is active.
                update_score_text.run_if(in_state(StageUiPluginUpdateState::Active)),
            );
        // .add_plugins(PauseScreenPlugin);
    }
}

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
/// Stage UI enable/disable flag.
pub enum StageUiPluginUpdateState {
    #[default]
    Inactive,
    Active,
}
