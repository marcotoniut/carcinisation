pub mod hud;

use bevy::prelude::*;

use self::hud::HudPlugin;

pub struct StageUiPlugin;

impl Plugin for StageUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(HudPlugin);
        // .add_plugins(PauseScreenPlugin);
    }
}
