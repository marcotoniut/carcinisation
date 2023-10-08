pub mod events;
pub mod resources;
pub mod systems;

use bevy::prelude::*;

use self::{
    events::{GameOver, GameRestart},
    systems::*,
};

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<GamePluginUpdateState>()
            .add_systems(OnEnter(GamePluginUpdateState::Active), start_stage)
            .add_event::<GameOver>()
            .add_event::<GameRestart>();
    }
}

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
pub enum GamePluginUpdateState {
    #[default]
    Inactive,
    Active,
}
