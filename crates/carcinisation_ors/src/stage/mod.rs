//! Stage gameplay orchestration: spawns enemies, drives progression, and renders UI overlays.
//!
//! # Position spaces
//!
//! Entities participate in two position spaces:
//!
//! - **World space.** `WorldPos` (and derived `CxPosition`). Read by
//!   simulation — physics, AI, spawn placement. Never write projection-adjusted,
//!   parallax-adjusted, or any visual-space coordinate into `WorldPos`.
//!
//! - **Visual space.** World position plus `CxPresentationTransform.visual_offset`
//!   (for rendering) or `.collision_offset` (for collision). Read by rendering,
//!   collision hit-detection, debug overlays.
//!
//! All visual displacement (parallax, projection, future knockback/hit-flash)
//! lives in `CxPresentationTransform`. Collision-readable presentation state is
//! composed in `Update`; composed sprite parts/visibility are then written in
//! `PostUpdate`. Simulation systems write world-space only.
//!
//! For composed enemies, spawn-time priming is explicit: scale and offsets must
//! already match the runtime presentation rules before the root is allowed to
//! show its first visible frame. Spawn must not rely on same-frame systems to
//! fix presentation later, reveal must never repair presentation, and runtime
//! systems only maintain already-correct state.

pub mod attack;
pub mod bundles;
pub mod collision;
pub mod components;
pub mod data;
pub mod depth_debug;
pub mod depth_scale;
pub mod destructible;
pub mod enemy;
pub mod floors;
pub mod messages;
pub mod parallax;
pub mod pickup;
pub mod player;
pub mod projection;
pub mod resources;
pub mod restart;
pub mod spawn_placement;
pub mod sprite_names;
mod systems;
pub mod ui;
pub use systems::spawn::check_step_spawn;

#[cfg(debug_assertions)]
use self::systems::debug_visibility_hierarchy;
use self::{
    attack::AttackPlugin,
    depth_scale::apply_depth_fallback_scale,
    destructible::DestructiblePlugin,
    enemy::EnemyPlugin,
    enemy::composed::{apply_composed_part_damage, check_composed_damage_flicker_taken},
    messages::{
        ComposedAnimationCueMessage, DamageMessage, DepthChangedMessage, NextStepEvent,
        PartDamageMessage, StageClearedEvent, StageDeathEvent, StageSpawnEvent, StageStartupEvent,
    },
    parallax::{
        ActiveParallaxAttenuation, compose_presentation_offsets,
        update_active_parallax_attenuation, update_parallax_offset,
    },
    pickup::systems::health::{
        mark_pickup_feedback_for_despawn, pickup_health, tick_pickup_drop_physics,
        update_pickup_feedback_glitter, update_pickup_feedback_scale,
    },
    pickup::visual::assemble_pickup_visuals,
    player::{PlayerPlugin, systems::camera::camera_shake},
    resources::{StageActionTimer, StageGravity, StageProgress, StageTimeDomain},
    restart::StageRestartPlugin,
    systems::{
        camera::{
            check_in_view, check_outside_view, initialise_camera_from_stage, update_camera_pos_x,
            update_camera_pos_y, update_lateral_view_offset,
        },
        check_movement_step_reached, check_stage_death, check_stage_step_timer,
        check_staged_cleared, check_stop_step_finished_by_duration,
        damage::{add_invert_filter, check_damage_flicker_taken, on_damage, remove_invert_filter},
        initialise_cinematic_step, initialise_movement_step, initialise_stop_step,
        movement::{
            check_jump_tween_finished, check_jump_tween_z_finished, check_linear_tween_finished,
            check_linear_tween_x_finished, check_linear_tween_y_finished, circle_around,
            derive_enemy_depth_from_continuous, sync_enemy_continuous_depth_from_targeting_z,
            update_enemy_pos_x, update_enemy_pos_y, update_non_enemy_depth_from_targeting_z,
        },
        on_death, on_next_step_cleanup_cinematic_step, on_next_step_cleanup_movement_step,
        on_next_step_cleanup_stop_step, on_stage_cleared, read_step_trigger,
        setup::on_stage_startup,
        spawn::{check_dead_drop, on_stage_spawn},
        tick_stage_step_timer, toggle_game, update_active_floor_layout, update_active_floors,
        update_active_projection, update_cinematic_step, update_stage,
        update_stage_time_should_run,
    },
    ui::{StageUiPlugin, pause_menu::pause_menu_renderer},
};
use crate::stubs::{
    GameProgressState, PositionSyncSystems, check_despawn_after_delay, delay_despawn,
};
use activable::{Activable, ActivableAppExt, activate_system, deactivate_system};
#[cfg(feature = "hot_reload")]
use bevy::asset::AssetEvent;
use bevy::prelude::*;
use bevy_common_assets::ron::RonAssetPlugin;
use carapace::prelude::WorldPos;
use carcinisation_core::core::event::on_trigger_write_event;
#[cfg(debug_assertions)]
use carcinisation_core::core::time::TimeMultiplier;
use carcinisation_core::core::time::{TimeShouldRun, tick_time};
use cween::{
    linear::{
        LinearTween2DPlugin, LinearTweenPlugin, LinearTweenSystems,
        components::{TargetingValueX, TargetingValueY, TargetingValueZ},
    },
    pursue::PursueMovementPlugin,
};
use data::StageData;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
/// Systems that load stage data and assets before play begins.
pub struct LoadingSystems;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
/// Systems that build out level entities once resources are available.
pub struct BuildingSystems;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
/// Systems that produce collision-readable state: presentation offsets and
/// composed collision volumes.  Any system that reads
/// `CxPresentationTransform.collision_offset` or `ComposedCollisionState` for
/// hit detection should run `.after(CollisionStateSystems)`.
pub struct CollisionStateSystems;

/// Function-pointer hooks that downstream systems use to activate/deactivate
/// the stage, game, and menu plugins without knowing the concrete types.
///
/// Inserted by `StagePlugin<G, M>::build()` so that systems like
/// `handle_game_over_screen_continue` and `handle_stage_restart` can call
/// the right `activate`/`deactivate` without carrying generic parameters.
#[derive(Resource)]
pub struct StageHooks {
    /// `activate::<StagePlugin<G, M>>`
    pub activate_stage: fn(&mut Commands),
    /// `deactivate::<StagePlugin<G, M>>`
    pub deactivate_stage: fn(&mut Commands),
    /// `deactivate::<G>` (the game plugin)
    pub deactivate_game: fn(&mut Commands),
    /// `activate::<M>` (the main-menu plugin)
    pub activate_menu: fn(&mut Commands),
    /// Fire a visual transition (venetian wipe, etc).
    /// Default: no-op. App provides the real implementation.
    pub trigger_transition: fn(&mut Commands, &carcinisation_cutscene::data::TransitionRequest),
}

/// Registers all stage-related plugins, assets, events, and frame drives.
///
/// `G` is the *game plugin* `Activable` marker (deactivated on game-over exit).
/// `M` is the *main-menu plugin* `Activable` marker (activated on game-over exit).
#[derive(Activable)]
pub struct StagePlugin<G: Activable, M: Activable> {
    _phantom: std::marker::PhantomData<(G, M)>,
}

impl<G: Activable, M: Activable> StagePlugin<G, M> {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<G: Activable, M: Activable> Default for StagePlugin<G, M> {
    fn default() -> Self {
        Self::new()
    }
}

/**
 * TODO
 * - implement a lifecycle state to indicate whether the plugin is active, inactive, (and perhaps initialising or cleaning up.)
 * - implement mapping of buttons exclusive to the plugin. (then we could have the menus create their own mappers.)
 */
impl<G: Activable, M: Activable> Plugin for StagePlugin<G, M> {
    #[allow(clippy::too_many_lines)]
    fn build(&self, app: &mut App) {
        #[cfg(debug_assertions)]
        app.insert_resource(TimeMultiplier::<StageTimeDomain>::new(1.));

        let app = app.add_plugins(RonAssetPlugin::<StageData>::new(&["sg.ron"]));

        app.insert_resource(data::OrsGameplayConfig::load());

        #[cfg(feature = "hot_reload")]
        app.add_plugins(carcinisation_core::dev_reload::DevReloadPlugin);

        #[cfg(feature = "hot_reload")]
        {
            carcinisation_core::watch_config!(app, "assets/config/player.ron");
            carcinisation_core::watch_config!(
                app,
                "assets/config/attacks/player_flamethrower_ors.ron"
            );
            carcinisation_core::watch_config!(app, "assets/config/attacks/blood_shot.ron");
            carcinisation_core::watch_config!(app, "assets/config/attacks/spider_shot.ron");
            carcinisation_core::watch_config!(app, "assets/config/attacks/boulder_throw.ron");
            carcinisation_core::watch_config!(app, "assets/config/ors/gameplay.ron");

            app.add_systems(Update, log_stage_data_asset_changes);

            carcinisation_core::reload_ron_system!(
                reload_player_config,
                player::config::PlayerConfig,
                "assets/config/player.ron"
            );
            carcinisation_core::reload_ron_system!(
                reload_flamethrower_config,
                player::flamethrower::FlamethrowerConfig,
                "assets/config/attacks/player_flamethrower_ors.ron"
            );
            carcinisation_core::reload_ron_system!(
                reload_blood_shot_config,
                attack::data::blood_shot::BloodShotConfig,
                "assets/config/attacks/blood_shot.ron"
            );
            carcinisation_core::reload_ron_system!(
                reload_spider_shot_config,
                attack::data::spider_shot::SpiderShotConfig,
                "assets/config/attacks/spider_shot.ron"
            );
            carcinisation_core::reload_ron_system!(
                reload_boulder_throw_config,
                attack::data::boulder_throw::BoulderThrowConfig,
                "assets/config/attacks/boulder_throw.ron"
            );
            carcinisation_core::reload_ron_system!(
                reload_ors_gameplay_config,
                data::OrsGameplayConfig,
                "assets/config/ors/gameplay.ron"
            );
            app.add_systems(
                Update,
                (
                    reload_player_config,
                    reload_flamethrower_config,
                    reload_blood_shot_config,
                    reload_spider_shot_config,
                    reload_boulder_throw_config,
                    reload_ors_gameplay_config,
                ),
            );
        }

        #[cfg(debug_assertions)]
        app.add_systems(Update, debug_visibility_hierarchy);

        #[cfg(debug_assertions)]
        app.add_plugins(depth_debug::DepthDebugPlugin);

        #[cfg(debug_assertions)]
        app.add_active_systems::<Self, _>(
            systems::debug_spawn::debug_keyboard_spawn_enemies
                .run_if(in_state(StageProgressState::Running)),
        );

        app.insert_resource(StageHooks {
            activate_stage: activable::activate::<Self>,
            deactivate_stage: activable::deactivate::<Self>,
            deactivate_game: activable::deactivate::<G>,
            activate_menu: activable::activate::<M>,
            trigger_transition: |_commands, _request| {
                // No-op default. App overrides via StageHooks::with_transition().
            },
        });

        app
            // Core stage state/resources that every sub-system relies on.
            .init_state::<StageProgressState>()
            .init_resource::<StageActionTimer>()
            .init_resource::<Time<StageTimeDomain>>()
            .init_resource::<TimeShouldRun<StageTimeDomain>>()
            .init_resource::<StageProgress>()
            .init_resource::<StageGravity>()
            .init_resource::<floors::ActiveSurfaceLayout>()
            .init_resource::<floors::ActiveFloors>()
            .init_resource::<resources::ActiveProjection>()
            .init_resource::<resources::ProjectionView>()
            .init_resource::<ActiveParallaxAttenuation>()
            .register_type::<ActiveParallaxAttenuation>()
            .register_type::<parallax::ParallaxOffset>()
            .insert_resource(depth_scale::DepthScaleConfig::load_or_default())
            .configure_sets(Update, CollisionStateSystems)
            // Message streams for the combat/progression loop.
            .add_message::<DamageMessage>()
            .add_message::<PartDamageMessage>()
            .add_message::<ComposedAnimationCueMessage>()
            .add_message::<DepthChangedMessage>()
            .add_message::<StageDeathEvent>()
            .add_observer(on_death)
            .add_message::<NextStepEvent>()
            .add_observer(on_next_step_cleanup_movement_step)
            .add_observer(on_next_step_cleanup_cinematic_step)
            .add_observer(on_next_step_cleanup_stop_step)
            .add_message::<StageStartupEvent>()
            .add_observer(on_stage_startup)
            .add_message::<StageSpawnEvent>()
            .add_observer(on_stage_spawn)
            .add_message::<StageClearedEvent>()
            .add_observer(on_stage_cleared)
            .add_observer(on_trigger_write_event::<StageClearedEvent>)
            // Checkpoint resume is handled via the `from_checkpoint` flag on `StageStartupEvent`.
            .on_active::<Self, _>((
                activate_system::<AttackPlugin>,
                activate_system::<DestructiblePlugin>,
                activate_system::<EnemyPlugin>,
                activate_system::<PlayerPlugin>,
                activate_system::<StageUiPlugin>,
            ))
            .on_inactive::<Self, _>((
                deactivate_system::<AttackPlugin>,
                deactivate_system::<DestructiblePlugin>,
                deactivate_system::<EnemyPlugin>,
                deactivate_system::<PlayerPlugin>,
                deactivate_system::<StageUiPlugin>,
            ))
            // Pause/unpause the player when the game state toggles.
            .add_systems(
                OnEnter(GameProgressState::Paused),
                deactivate_system::<PlayerPlugin>,
            )
            .add_systems(
                OnExit(GameProgressState::Paused),
                activate_system::<PlayerPlugin>,
            )
            // Shared movement helpers (linear/pursue) reused by multiple enemy types.
            .add_plugins(PursueMovementPlugin::<StageTimeDomain, WorldPos>::default())
            .add_plugins(LinearTweenPlugin::<StageTimeDomain, TargetingValueX>::default())
            .add_plugins(LinearTweenPlugin::<StageTimeDomain, TargetingValueY>::default())
            .add_plugins(LinearTweenPlugin::<StageTimeDomain, TargetingValueZ>::default())
            .add_plugins(LinearTween2DPlugin::<
                StageTimeDomain,
                TargetingValueX,
                TargetingValueY,
            >::default())
            .add_plugins(AttackPlugin)
            .add_plugins(DestructiblePlugin)
            .add_plugins(EnemyPlugin)
            .add_plugins(PlayerPlugin)
            .add_plugins(StageRestartPlugin)
            .add_plugins(StageUiPlugin)
            .add_active_systems_in::<Self, _>(
                FixedUpdate,
                (
                    (
                        tick_time::<Fixed, StageTimeDomain>.before(LinearTweenSystems),
                        tick_stage_step_timer,
                        delay_despawn::<StageTimeDomain>,
                        check_despawn_after_delay::<StageTimeDomain>,
                    ),
                    (
                        (
                            sync_enemy_continuous_depth_from_targeting_z,
                            derive_enemy_depth_from_continuous,
                        )
                            .chain(),
                        update_non_enemy_depth_from_targeting_z,
                        circle_around,
                        (
                            (
                                check_linear_tween_x_finished,
                                check_linear_tween_y_finished,
                                check_jump_tween_z_finished,
                            ),
                            (check_linear_tween_finished, check_jump_tween_finished),
                        )
                            .chain(),
                    )
                        .after(LinearTweenSystems),
                ),
            )
            // Depth-fallback scale + parallax composition run in Update
            // (after PositionSyncSystems) so that update_composed_enemy_visuals
            // reads the current frame's collision_offset. This ordering keeps
            // maintained presentation state current; it is not allowed to act
            // as a same-frame repair path for newly spawned composed roots.
            // First-visible-frame correctness comes from spawn-time priming
            // alone, not from any same-frame ordering accident.
            .add_active_systems::<Self, _>(
                (
                    apply_depth_fallback_scale.in_set(CollisionStateSystems),
                    update_parallax_offset
                        .after(apply_depth_fallback_scale)
                        .after(update_lateral_view_offset),
                    compose_presentation_offsets
                        .after(update_parallax_offset)
                        .after(apply_depth_fallback_scale)
                        .in_set(CollisionStateSystems),
                )
                    .after(PositionSyncSystems),
            )
            .add_active_systems::<Self, _>((
                update_stage,
                update_active_projection.after(update_stage),
                update_active_parallax_attenuation.after(update_stage),
                update_active_floor_layout.after(update_stage),
                update_active_floors.after(update_stage),
                update_stage_time_should_run.after(update_stage),
                update_lateral_view_offset
                    .after(update_camera_pos_x)
                    .after(update_active_projection),
                (
                    (
                        // Camera
                        initialise_camera_from_stage,
                        camera_shake,
                        check_in_view,
                        check_outside_view,
                        update_camera_pos_x,
                        update_camera_pos_y,
                        update_enemy_pos_x,
                        update_enemy_pos_y,
                    ),
                    (
                        // Pickup
                        assemble_pickup_visuals,
                        pickup_health.after(assemble_pickup_visuals),
                        tick_pickup_drop_physics,
                        update_pickup_feedback_glitter,
                        update_pickup_feedback_scale,
                        mark_pickup_feedback_for_despawn,
                    ),
                    (
                        // Stage
                        read_step_trigger.run_if(in_state(StageProgressState::Running)),
                        check_stage_step_timer,
                        check_staged_cleared.run_if(in_state(StageProgressState::Running)),
                        check_step_spawn,
                        check_stage_death,
                    ),
                    (
                        // Damage
                        (
                            apply_composed_part_damage,
                            on_damage,
                            check_composed_damage_flicker_taken,
                            check_damage_flicker_taken,
                        )
                            .chain(),
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
                ),
            ))
            // Screen render/despawn systems are owned by their respective
            // plugins (DeathScreenPlugin, GameOverScreenPlugin,
            // ClearedScreenPlugin) and gated via StageUiPlugin.  Do NOT
            // re-register them here — that would cause double execution.
            .add_active_systems::<Self, _>((
                // Pause menu
                pause_menu_renderer,
                toggle_game.run_if(in_state(StageProgressState::Running)),
            ));
    }
}

/// Logs when a `.sg.ron` stage asset is modified on disk (`hot_reload` only).
///
/// With `bevy/file_watcher` enabled, the asset server detects file changes and
/// re-emits `AssetEvent::Modified`. This system surfaces those events so the
/// developer knows a reload was detected. Full subsystem rebuild on change is
/// planned for a later phase.
#[cfg(feature = "hot_reload")]
fn log_stage_data_asset_changes(mut events: MessageReader<AssetEvent<StageData>>) {
    for event in events.read() {
        match event {
            AssetEvent::Modified { id } => {
                info!("Stage data asset modified (id={id:?}) — restart stage to apply changes");
            }
            AssetEvent::LoadedWithDependencies { id } => {
                debug!("Stage data asset loaded (id={id:?})");
            }
            _ => {}
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Activable)]
    struct MockGame;
    #[derive(Activable)]
    struct MockMenu;

    /// Verify `StageHooks` captures the correct monomorphized function pointers.
    #[test]
    fn stage_hooks_captures_correct_function_pointers() {
        let hooks = StageHooks {
            activate_stage: activable::activate::<StagePlugin<MockGame, MockMenu>>,
            deactivate_stage: activable::deactivate::<StagePlugin<MockGame, MockMenu>>,
            deactivate_game: activable::deactivate::<MockGame>,
            activate_menu: activable::activate::<MockMenu>,
            trigger_transition: |_commands, _request| {},
        };

        // Verify the pointers resolve to the expected functions.
        let expected_activate: fn(&mut Commands) =
            activable::activate::<StagePlugin<MockGame, MockMenu>>;
        let expected_deactivate_game: fn(&mut Commands) = activable::deactivate::<MockGame>;
        let expected_activate_menu: fn(&mut Commands) = activable::activate::<MockMenu>;

        assert_eq!(
            hooks.activate_stage as usize, expected_activate as usize,
            "activate_stage should point to activate::<StagePlugin<MockGame, MockMenu>>"
        );
        assert_eq!(
            hooks.deactivate_game as usize, expected_deactivate_game as usize,
            "deactivate_game should point to deactivate::<MockGame>"
        );
        assert_eq!(
            hooks.activate_menu as usize, expected_activate_menu as usize,
            "activate_menu should point to activate::<MockMenu>"
        );
    }

    /// Default transition handler is a no-op (doesn't panic).
    #[test]
    fn default_transition_handler_is_noop() {
        let hooks = StageHooks {
            activate_stage: activable::activate::<StagePlugin<MockGame, MockMenu>>,
            deactivate_stage: activable::deactivate::<StagePlugin<MockGame, MockMenu>>,
            deactivate_game: activable::deactivate::<MockGame>,
            activate_menu: activable::activate::<MockMenu>,
            trigger_transition: |_commands, _request| {},
        };

        // Calling the no-op handler with a real TransitionRequest must not panic.
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.update();
        let mut commands = app.world_mut().commands();
        (hooks.trigger_transition)(
            &mut commands,
            &carcinisation_cutscene::data::TransitionRequest::Venetian,
        );
    }
}
