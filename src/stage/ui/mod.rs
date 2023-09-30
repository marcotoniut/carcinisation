pub mod cleared_screen;
pub mod components;
pub mod game_over_screen;
pub mod hud;
pub mod pause_menu;
pub mod systems;

use bevy::prelude::*;

use self::{hud::HudPlugin, systems::update_score_text};

pub struct StageUiPlugin;

impl Plugin for StageUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(HudPlugin)
            .add_systems(Update, update_score_text);
        // .add_plugins(PauseScreenPlugin);
    }
}
