pub mod components;
pub mod events;
pub mod systems;

use self::systems::*;
use bevy::prelude::*;

pub struct LetterboxPlugin;

impl Plugin for LetterboxPlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<LetterboxPluginUpdateState>()
            .add_systems(OnEnter(LetterboxPluginUpdateState::Active), on_startup)
            .add_systems(OnEnter(LetterboxPluginUpdateState::Inactive), on_shutdown);
        // .add_system(systems::update_letterbox)
    }
}

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
pub enum LetterboxPluginUpdateState {
    Inactive,
    #[default]
    Active,
}
