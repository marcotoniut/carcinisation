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
mod systems;
pub mod ui;

use self::{
    attack::AttackPlugin,
    components::placement::RailPosition,
    destructible::DestructiblePlugin,
    enemy::EnemyPlugin,
    events::*,
    pickup::systems::health::pickup_health,
    player::PlayerPlugin,
    resources::{StageActionTimer, StageProgress, StageTime},
    systems::{
        camera::*,
        damage::*,
        movement::*,
        setup::on_stage_startup,
        spawn::{check_dead_drop, check_step_spawn, on_stage_spawn},
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
    core::{
        event::on_trigger_write_event,
        time::{tick_time, TimeMultiplier},
    },
    globals::mark_for_despawn_by_query_system,
    plugins::movement::{
        linear::{
            components::{TargetingPositionX, TargetingPositionY, TargetingPositionZ},
            LinearMovement2DPlugin, LinearMovementPlugin,
        },
        pursue::PursueMovementPlugin,
    },
    systems::{check_despawn_after_delay, delay_despawn},
};
use bevy::prelude::*;
use bevy_common_assets::ron::RonAssetPlugin;
use data::StageData;
use pickup::systems::health::PickupDespawnFilter;
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

        app.add_plugins(RonAssetPlugin::<StageData>::new(&["sg.ron"]))
            .init_state::<StagePluginUpdateState>()
            .init_state::<StageProgressState>()
            .init_resource::<StageActionTimer>()
            .init_resource::<StageTime>()
            .init_resource::<StageProgress>()
            .add_event::<DamageEvent>()
            .add_event::<DepthChangedEvent>()
            .add_event::<StageDeathEvent>()
            .add_observer(on_death)
            .add_event::<NextStepEvent>()
            .add_observer(on_next_step_cleanup_movement_step)
            .add_observer(on_next_step_cleanup_cinematic_step)
            .add_observer(on_next_step_cleanup_stop_step)
            .add_event::<StageStartupTrigger>()
            .add_observer(on_stage_startup)
            .add_event::<StageSpawnTrigger>()
            .add_observer(on_stage_spawn)
            .add_event::<StageClearedTrigger>()
            .add_observer(on_stage_cleared)
            .add_observer(on_trigger_write_event::<StageClearedTrigger>)
            // TODO .add_observer(on_startup_from_checkpoint))
            .add_systems(OnEnter(StagePluginUpdateState::Active), on_active)
            .add_systems(OnEnter(StagePluginUpdateState::Inactive), on_inactive)
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
                            check_stage_step_timer,
                            check_staged_cleared,
                            check_step_spawn,
                            check_stage_death,
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
                            (on_damage, check_damage_flicker_taken).chain(),
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
