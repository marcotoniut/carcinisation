pub mod cleared_screen;
pub mod game_over_screen;
pub mod hud;
pub mod pause_menu;

use bevy::prelude::*;

use self::hud::HudPlugin;

pub struct StageUiPlugin;

impl Plugin for StageUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(HudPlugin);
        // .add_plugins(PauseScreenPlugin);
    }
}
