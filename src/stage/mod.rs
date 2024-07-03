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
pub mod systems;
pub mod ui;

use self::{
    attack::AttackPlugin,
    components::placement::RailPosition,
    destructible::DestructiblePlugin,
    enemy::EnemyPlugin,
    events::*,
    pickup::systems::health::pickup_health,
    player::{events::CameraShakeEvent, PlayerPlugin},
    resources::{StageActionTimer, StageProgress, StageTime},
    systems::{
        camera::*,
        damage::*,
        movement::*,
        setup::on_startup,
        spawn::{check_dead_drop, check_step_spawn, read_stage_spawn_trigger},
        state::{on_active, on_inactive},
        *,
    },
    ui::{
        cleared_screen::{despawn_cleared_screen, render_cleared_screen},
        death_screen::{despawn_death_screen, render_death_screen},
        game_over_screen::{despawn_game_over_screen, render_game_over_screen},
        pause_menu::pause_menu_renderer,
        StageUiPlugin,
    },
};
use crate::{
    core::time::{tick_time, TimeMultiplier},
    game::events::GameOverEvent,
    globals::mark_for_despawn_by_query_system,
    plugins::movement::{
        linear::{
            components::{
                LinearTargetReached, TargetingPositionX, TargetingPositionY, TargetingPositionZ,
            },
            LinearMovement2DPlugin, LinearMovementPlugin,
        },
        pursue::PursueMovementPlugin,
    },
    systems::{check_despawn_after_delay, delay_despawn},
};
use bevy::prelude::*;
use bevy_common_assets::ron::RonAssetPlugin;
use data::StageData;
use pickup::{components::PickupFeedback, systems::health::PickupDespawnFilter};
use seldom_pixel::prelude::PxSubPosition;

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
        #[cfg(debug_assertions)]
        app.insert_resource(TimeMultiplier::<StageTime>::new(1.));

        app.init_state::<StagePluginUpdateState>()
            .init_resource::<StageActionTimer>()
            .init_resource::<StageTime>()
            .init_resource::<StageProgress>()
            .init_state::<StageProgressState>()
            .add_systems(OnEnter(StagePluginUpdateState::Active), on_active)
            .add_systems(OnEnter(StagePluginUpdateState::Inactive), on_inactive)
            .add_plugins(RonAssetPlugin::<StageData>::new(&["sg.ron"]))
            .add_event::<CameraShakeEvent>()
            .add_event::<DamageEvent>()
            .add_event::<DepthChangedEvent>()
            .add_event::<StageClearedEvent>()
            .add_event::<StageDeathEvent>()
            .add_event::<StageSpawnEvent>()
            .add_event::<StageStartupEvent>()
            .add_event::<NextStepEvent>()
            // TODO temporary
            .add_event::<GameOverEvent>()
            .add_plugins(PursueMovementPlugin::<StageTime, RailPosition>::default())
            .add_plugins(PursueMovementPlugin::<StageTime, PxSubPosition>::default())
            .add_plugins(LinearMovementPlugin::<StageTime, TargetingPositionX>::default())
            .add_plugins(LinearMovementPlugin::<StageTime, TargetingPositionY>::default())
            .add_plugins(LinearMovementPlugin::<StageTime, TargetingPositionZ>::default())
            .add_plugins(LinearMovement2DPlugin::<
                StageTime,
                TargetingPositionX,
                TargetingPositionY,
            >::default())
            .add_plugins(AttackPlugin)
            .add_plugins(DestructiblePlugin)
            .add_plugins(EnemyPlugin)
            .add_plugins(PlayerPlugin)
            .add_plugins(StageUiPlugin)
            // .add_event::<StageSetupEvent>()
            // .add_event::<StageSetupFromCheckpointEvent>()
            // .add_systems(PreUpdate, (on_setup, on_setup_from_checkpoint))
            // TODO should this be only used when plugin is active?
            // Should initialisation functions be chained to startup?
            .add_systems(PostUpdate, on_startup)
            // // TEMP
            // .add_systems(
            //     Update,
            //     spawn_current_stage_bundle.run_if(in_state(GameProgressState::Loading)),
            // )
            .add_systems(
                Update,
                (
                    update_stage,
                    (
                        (
                            // Camera
                            check_in_view,
                            check_outside_view,
                            update_camera_pos_x,
                            update_camera_pos_y,
                        ),
                        (
                            // Pickup
                            pickup_health,
                            mark_for_despawn_by_query_system::<PickupDespawnFilter>,
                        ),
                        (
                            // Stage
                            tick_time::<StageTime>,
                            tick_stage_step_timer,
                            read_step_trigger,
                            read_stage_spawn_trigger,
                            check_stage_step_timer,
                            check_staged_cleared,
                            read_stage_cleared_trigger,
                            check_step_spawn,
                            check_stage_death,
                            read_stage_death_trigger,
                        ),
                        (
                            // Effects
                            delay_despawn::<StageTime>,
                            check_despawn_after_delay::<StageTime>,
                        ),
                        (
                            // Movement
                            update_depth,
                            circle_around,
                            (
                                (
                                    check_linear_movement_x_finished,
                                    check_linear_movement_y_finished,
                                ),
                                check_linear_movement_finished,
                            )
                                .chain(),
                        ),
                        (
                            // Damage
                            (check_damage_taken, check_damage_flicker_taken).chain(),
                            add_invert_filter,
                            remove_invert_filter,
                            check_dead_drop,
                        ),
                        (
                            (
                                initialise_cinematic_step,
                                initialise_movement_step,
                                initialise_stop_step,
                            ),
                            (
                                update_cinematic_step,
                                check_stop_step_finished_by_duration,
                                check_movement_step_reached,
                            ),
                            (
                                cleanup_movement_step,
                                cleanup_cinematic_step,
                                cleanup_stop_step,
                            ),
                        )
                            .chain(),
                    )
                        .run_if(in_state(StageProgressState::Running)),
                )
                    .run_if(in_state(StagePluginUpdateState::Active)),
            )
            .add_systems(
                Update,
                (
                    // Cleared screen
                    render_cleared_screen,
                    despawn_cleared_screen,
                    // Death screen
                    render_death_screen,
                    despawn_death_screen,
                    // Game Over screen
                    render_game_over_screen,
                    despawn_game_over_screen,
                    // Pause menu
                    pause_menu_renderer,
                    toggle_game,
                )
                    .run_if(in_state(StagePluginUpdateState::Active)),
            );
    }
}

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
pub enum StageProgressState {
    #[default]
    Initial,
    Running,
    Clear,
    Cleared,
    Death,
    GameOver,
}

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
pub enum StagePluginUpdateState {
    #[default]
    Inactive,
    Active,
}
