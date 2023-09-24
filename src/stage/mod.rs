pub mod bundles;
pub mod components;
pub mod data;
pub mod enemy;
pub mod events;
pub mod pickup;
pub mod player;
pub mod resources;
pub mod score;
pub mod star;
pub mod systems;
pub mod ui;

use bevy::prelude::*;

use self::{
    enemy::{
        systems::mosquito::{
            check_idle_mosquito, damage_on_reached, read_enemy_attack_depth_changed,
        },
        EnemyPlugin,
    },
    events::*,
    pickup::systems::health::pickup_health,
    player::{
        systems::camera::{camera_shake, trigger_shake},
        PlayerPlugin,
    },
    resources::{StageActionTimer, StageProgress, StageTime},
    score::{components::Score, ScorePlugin},
    systems::{movement::*, spawn::read_stage_spawn_trigger, *},
    ui::{pause_menu::pause_menu_renderer, StageUiPlugin},
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
            .add_state::<StageState>()
            .add_event::<DepthChanged>()
            .add_event::<GameOver>()
            .add_event::<StageStepTrigger>()
            .add_event::<StageSpawnTrigger>()
            .init_resource::<StageActionTimer>()
            .init_resource::<StageTime>()
            .init_resource::<Score>()
            .init_resource::<StageProgress>()
            .add_plugins(EnemyPlugin)
            .add_plugins(PlayerPlugin)
            .add_plugins(ScorePlugin)
            .add_plugins(StageUiPlugin)
            // .add_plugins(StarPlugin)
            .add_systems(PostStartup, setup_stage.in_set(LoadingSystemSet))
            .add_systems(
                Update,
                spawn_current_stage_bundle.run_if(in_state(GameState::Loading)),
            )
            .add_systems(
                Update,
                (
                    update_stage,
                    (
                        // Player
                        camera_shake,
                        trigger_shake,
                        //
                        increment_elapsed,
                        tick_stage_step_timer,
                        read_stage_step_trigger,
                        read_stage_spawn_trigger,
                        check_stage_step_timer,
                        check_staged_cleared,
                        pickup_health,
                        damage_on_reached,
                        // Enemy
                        check_idle_mosquito,
                        read_enemy_attack_depth_changed,
                        // Movement
                        advance_incoming,
                        check_depth_reached,
                        update_depth,
                        advance_line,
                        check_line_target_x_reached,
                        check_line_target_y_reached,
                        check_line_target_reached,
                        circle_around,
                    )
                        .run_if(in_state(StageState::Running)),
                )
                    .run_if(in_state(GameState::Running)),
            )
            // .add_systems(Update, run_timer)
            .add_systems(Update, toggle_game.run_if(in_state(AppState::Game)))
            .add_systems(Update, pause_menu_renderer.run_if(in_state(AppState::Game)))
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

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
pub enum StageState {
    #[default]
    Initial,
    Running,
    Clear,
    Cleared,
}
