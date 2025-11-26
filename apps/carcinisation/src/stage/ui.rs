//! Stage-specific UI overlays: HUD, cleared/death/game-over screens, and state gating.

pub mod cleared_screen;
pub mod components;
pub mod death_screen;
pub mod game_over_screen;
pub mod hud;
pub mod pause_menu;
mod systems;

use self::{
    cleared_screen::ClearedScreenPlugin, death_screen::DeathScreenPlugin,
    game_over_screen::GameOverScreenPlugin, hud::HudPlugin, systems::update_score_text,
};
use activable::{activate_system, deactivate_system, Activable, ActivableAppExt};
use bevy::prelude::*;

/// Registers all stage UI sub-plugins and manages their active state.
#[derive(Activable)]
pub struct StageUiPlugin;

impl Plugin for StageUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(HudPlugin)
            .add_plugins((ClearedScreenPlugin, DeathScreenPlugin, GameOverScreenPlugin))
            .on_active::<StageUiPlugin, _>(activate_system::<HudPlugin>)
            .on_inactive::<StageUiPlugin, _>(deactivate_system::<HudPlugin>)
            .add_active_systems::<StageUiPlugin, _>(update_score_text);
    }
}
