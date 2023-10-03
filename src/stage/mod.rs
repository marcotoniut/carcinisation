pub mod attack;
pub mod bundles;
pub mod components;
pub mod data;
pub mod destructible;
pub mod enemy;
pub mod events;
pub mod pickup;
pub mod player;
pub mod resources;
pub mod score;
pub mod systems;
pub mod ui;

use bevy::prelude::*;
use seldom_pixel::prelude::PxSubPosition;

use self::{
    attack::AttackPlugin,
    components::placement::RailPosition,
    destructible::DestructiblePlugin,
    enemy::EnemyPlugin,
    events::*,
    pickup::systems::health::{mark_despawn_pickup_feedback, pickup_health},
    player::{
        events::CameraShakeEvent,
        systems::camera::{camera_shake, trigger_shake},
        PlayerPlugin,
    },
    resources::{StageActionTimer, StageProgress, StageStepSpawner, StageTime},
    score::{components::Score, ScorePlugin},
    systems::{
        camera::*,
        damage::*,
        movement::*,
        spawn::{check_dead_drop, check_step_spawn, read_stage_spawn_trigger},
        *,
    },
    ui::{
        cleared_screen::{despawn_cleared_screen, render_cleared_screen},
        game_over_screen::{despawn_game_over_screen, render_game_over_screen},
        pause_menu::pause_menu_renderer,
        StageUiPlugin,
    },
};
use crate::{
    cinemachine::{cinemachine::CinemachineScene, render_cutscene},
    game::events::GameOver,
    plugins::movement::{
        linear::{
            components::{TargetingPositionX, TargetingPositionY, TargetingPositionZ},
            LinearMovementPlugin,
        },
        pursue::PursueMovementPlugin,
    },
    AppState,
};

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct LoadingSystemSet;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct BuildingSystemSet;

pub struct StagePlugin;

/**
 * TODO
 * - implement a lifecycle state to indicate whether the plugin is active, inactive, (and perhaps initialising or cleaning up.)
 * - implement mapping of buttons exclusive to the plugin. (then we could have the menus create their own mappers.)
 */
impl Plugin for StagePlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<GameState>()
            .add_state::<StageState>()
            .add_event::<CameraShakeEvent>()
            .add_event::<DamageEvent>()
            .add_event::<DepthChangedEvent>()
            .add_event::<StageClearedEvent>()
            .add_event::<StageGameOverEvent>()
            .add_event::<StageSpawnEvent>()
            .add_event::<StageStepEvent>()
            // TODO temporary
            .add_event::<GameOver>()
            .init_resource::<StageActionTimer>()
            .init_resource::<StageTime>()
            .init_resource::<Score>()
            .init_resource::<StageProgress>()
            .init_resource::<StageStepSpawner>()
            .init_resource::<CinemachineScene>()
            .add_plugins(PursueMovementPlugin::<StageTime, RailPosition>::default())
            .add_plugins(PursueMovementPlugin::<StageTime, PxSubPosition>::default())
            .add_plugins(LinearMovementPlugin::<StageTime, TargetingPositionX>::default())
            .add_plugins(LinearMovementPlugin::<StageTime, TargetingPositionY>::default())
            .add_plugins(LinearMovementPlugin::<StageTime, TargetingPositionZ>::default())
            .add_plugins(AttackPlugin)
            .add_plugins(DestructiblePlugin)
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
                        (
                            // Camera
                            check_in_view,
                            check_outside_view,
                        ),
                        (
                            // Player
                            camera_shake,
                            trigger_shake,
                        ),
                        (
                            // Pickup
                            pickup_health,
                            mark_despawn_pickup_feedback,
                        ),
                        (
                            // Stage
                            increment_elapsed,
                            tick_stage_time,
                            tick_stage_step_timer,
                            read_stage_step_trigger,
                            read_stage_spawn_trigger,
                            check_stage_step_timer,
                            check_staged_cleared,
                            read_stage_cleared_trigger,
                            check_step_spawn,
                            check_stage_game_over,
                            read_stage_game_over_trigger,
                            // TEMP
                            check_stage_game_over,
                            read_stage_game_over_trigger,
                        ),
                        (
                            // Movement
                            update_position_x,
                            update_position_y,
                            update_depth,
                            circle_around,
                            check_linear_movement_finished,
                        ),
                        (
                            // Damage
                            (check_damage_taken, check_damage_flicker_taken).chain(),
                            add_invert_filter,
                            remove_invert_filter,
                            check_dead_drop,
                        ),
                    )
                        .run_if(in_state(StageState::Running)),
                )
                    .run_if(in_state(GameState::Running))
                    .run_if(in_state(AppState::Game)),
            )
            // .add_systems(Update, run_timer)
            .add_systems(Update, toggle_game.run_if(in_state(AppState::Game)))
            .add_systems(
                Update,
                (
                    // Cleared screen
                    render_cleared_screen,
                    despawn_cleared_screen,
                    // Game Over screen
                    render_game_over_screen,
                    despawn_game_over_screen,
                )
                    .run_if(in_state(GameState::Running))
                    .run_if(in_state(AppState::Game)),
            )
            .add_systems(Update, pause_menu_renderer.run_if(in_state(AppState::Game)))
            .add_systems(Update, render_cutscene.run_if(in_state(AppState::Game)))
            .add_systems(OnEnter(AppState::Game), resume_game);
    }
}

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
pub enum GameState {
    #[default]
    Loading,
    Running,
    Paused,
    Cutscene,
}

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
pub enum StageState {
    #[default]
    Initial,
    Running,
    Clear,
    Cleared,
    // TODO temporary (for the jame)
    GameOver,
}
