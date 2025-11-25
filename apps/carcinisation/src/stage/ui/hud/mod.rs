pub mod components;
pub mod spawn;
mod systems;

use self::systems::update::*;
use activable::{Activable, ActivableAppExt};
use bevy::prelude::*;

#[derive(Activable)]
pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        // app.add_active_systems::<HudPlugin, _>(update_health_text);
    }
}
