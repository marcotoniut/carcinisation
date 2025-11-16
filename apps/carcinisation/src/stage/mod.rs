//! Stage gameplay orchestration: spawns enemies, drives progression, and renders UI overlays.

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
pub mod restart;
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
    resources::{StageActionTimer, StageProgress, StageTimeDomain},
    restart::StageRestartPlugin,
    systems::{
        camera::*,
        damage::*,
        movement::*,
        setup::on_stage_startup,
        spawn::{check_dead_drop, check_step_spawn, on_stage_spawn},
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
    core::{event::on_trigger_write_event, time::TimeMultiplier},
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
use activable::{activate_system, deactivate_system, Activable, ActivableAppExt};
use bevy::prelude::*;
use bevy_common_assets::ron::RonAssetPlugin;
use data::StageData;
use pickup::systems::health::PickupDespawnFilter;
use seldom_pixel::prelude::PxSubPosition;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
/// Systems that load stage data and assets before play begins.
pub struct LoadingSystems;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
/// Systems that build out level entities once resources are available.
pub struct BuildingSystems;

/// Registers all stage-related plugins, assets, events, and frame drives.
#[derive(Activable)]
pub struct StagePlugin;

/**
 * TODO
 * - implement a lifecycle state to indicate whether the plugin is active, inactive, (and perhaps initialising or cleaning up.)
 * - implement mapping of buttons exclusive to the plugin. (then we could have the menus create their own mappers.)
 */
impl Plugin for StagePlugin {
    fn build(&self, app: &mut App) {
        #[cfg(debug_assertions)]
        app.insert_resource(TimeMultiplier::<StageTimeDomain>::new(1.));

        app.add_plugins(RonAssetPlugin::<StageData>::new(&["sg.ron"]))
            // Core stage state/resources that every sub-system relies on.
            .init_state::<StageProgressState>()
            .init_resource::<StageActionTimer>()
            .init_resource::<Time<StageTimeDomain>>()
            .init_resource::<StageProgress>()
            // Event streams for the combat/progression loop.
            .add_message::<DamageEvent>()
            .add_message::<DepthChangedEvent>()
            .add_message::<StageDeathEvent>()
            .add_observer(on_death)
            .add_message::<NextStepEvent>()
            .add_observer(on_next_step_cleanup_movement_step)
            .add_observer(on_next_step_cleanup_cinematic_step)
            .add_observer(on_next_step_cleanup_stop_step)
            .add_message::<StageStartupTrigger>()
            .add_observer(on_stage_startup)
            .add_message::<StageSpawnTrigger>()
            .add_observer(on_stage_spawn)
            .add_message::<StageClearedTrigger>()
            .add_observer(on_stage_cleared)
            .add_observer(on_trigger_write_event::<StageClearedTrigger>)
            // TODO .add_observer(on_startup_from_checkpoint))
            .on_active::<StagePlugin, _>((
                activate_system::<AttackPlugin>,
                activate_system::<DestructiblePlugin>,
                activate_system::<EnemyPlugin>,
                activate_system::<PlayerPlugin>,
                activate_system::<StageUiPlugin>,
            ))
            .on_inactive::<StagePlugin, _>((
                deactivate_system::<AttackPlugin>,
                deactivate_system::<DestructiblePlugin>,
                deactivate_system::<EnemyPlugin>,
                deactivate_system::<PlayerPlugin>,
                deactivate_system::<StageUiPlugin>,
            ))
            .add_active_systems_in::<StagePlugin, _>(
                PreUpdate,
                (tick_stage_time,).run_if(in_state(StageProgressState::Running)),
            )
            // Shared movement helpers (linear/pursue) reused by multiple enemy types.
            .add_plugins(PursueMovementPlugin::<StageTimeDomain, RailPosition>::default())
            .add_plugins(PursueMovementPlugin::<StageTimeDomain, PxSubPosition>::default())
            .add_plugins(LinearMovementPlugin::<StageTimeDomain, TargetingPositionX>::default())
            .add_plugins(LinearMovementPlugin::<StageTimeDomain, TargetingPositionY>::default())
            .add_plugins(LinearMovementPlugin::<StageTimeDomain, TargetingPositionZ>::default())
            .add_plugins(LinearMovement2DPlugin::<
                StageTimeDomain,
                TargetingPositionX,
                TargetingPositionY,
            >::default())
            .add_plugins(AttackPlugin)
            .add_plugins(DestructiblePlugin)
            .add_plugins(EnemyPlugin)
            .add_plugins(PlayerPlugin)
            .add_plugins(StageRestartPlugin)
            .add_plugins(StageUiPlugin)
            .add_active_systems::<StagePlugin, _>(
                // Primary stage tick, only when gameplay is active and running.
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
                            tick_stage_step_timer,
                            read_step_trigger,
                            check_stage_step_timer,
                            check_staged_cleared,
                            check_step_spawn,
                            check_stage_death,
                        ),
                        (
                            // Effects
                            delay_despawn::<StageTimeDomain>,
                            check_despawn_after_delay::<StageTimeDomain>,
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
                ),
            )
            .add_active_systems::<StagePlugin, _>(
                // Overlay/UI rendering keeps pace whenever the stage plugin is active.
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
                ),
            );
    }
}

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
/// High-level lifecycle for a stage run (used to gate systems).
pub enum StageProgressState {
    #[default]
    Initial,
    Running,
    Clear,
    Cleared,
    Death,
    GameOver,
}
