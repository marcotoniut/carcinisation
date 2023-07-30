pub mod hud;
pub mod pause_screen;

use bevy::prelude::*;

use self::{hud::HudPlugin, pause_screen::PauseScreenPlugin};

pub struct GameUiPlugin;

impl Plugin for GameUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(HudPlugin).add_plugins(PauseScreenPlugin);
    }
}
