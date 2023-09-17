pub mod bundles;
pub mod enemy;
pub mod player;
pub mod resources;
pub mod score;
pub mod star;
pub mod systems;
pub mod ui;

use bevy::prelude::*;

use self::{
    enemy::EnemyPlugin,
    player::PlayerPlugin,
    resources::{GameProgress, StageTimer},
    score::{components::Score, ScorePlugin},
    star::StarPlugin,
    systems::*,
    ui::StageUiPlugin,
};
use crate::{events::*, AppState};

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct LoadingSystemSet;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct BuildingSystemSet;

pub struct StagePlugin;

impl Plugin for StagePlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<GameState>()
            .add_event::<GameOver>()
            .init_resource::<StageTimer>()
            .init_resource::<Score>()
            .init_resource::<GameProgress>()
            .add_plugins(EnemyPlugin)
            .add_plugins(PlayerPlugin)
            .add_plugins(ScorePlugin)
            .add_plugins(StageUiPlugin)
            .add_plugins(StarPlugin)
            .add_systems(Startup, setup_stage.in_set(LoadingSystemSet))
            .add_systems(
                Update,
                spawn_current_stage_bundle.run_if(in_state(GameState::Loading)),
            )
            // .add_systems(Update, run_timer)
            .add_systems(Update, toggle_game.run_if(in_state(AppState::Game)))
            .add_systems(OnEnter(AppState::Game), resume_game);
    }
}

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
pub enum GameState {
    #[default]
    Loading,
    Running,
    Paused,
}
