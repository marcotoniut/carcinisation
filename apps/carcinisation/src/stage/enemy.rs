//! Enemy entity definitions, behaviours, and species-specific logic.

pub mod bundles;
pub mod components;
pub mod composed;
pub mod data;
pub mod entity;
pub mod mosquito;
pub mod mosquiton;
pub mod spidey;
mod systems;
pub mod tardigrade;

use self::{
    mosquito::systems::{
        assign_mosquito_animation, check_idle_mosquito, clear_finished_mosquito_attacks,
        despawn_dead_mosquitoes,
    },
    mosquiton::systems::{
        apply_mosquiton_falling_physics, assign_mosquiton_animation, despawn_dead_mosquitons,
        detect_part_breakage, trigger_mosquiton_authored_attack_cues,
        update_mosquiton_death_effect,
    },
    spidey::systems::{assign_spidey_animation, despawn_dead_spideys, update_spidey_death_effect},
    systems::{
        animation::on_composed_enemy_depth_changed,
        animation::on_enemy_depth_changed,
        behaviors::{
            check_no_behavior, cleanup_orphaned_tween_children, tick_enemy_behavior_timer,
            tied_components_enemy_current_behavior_circle_around,
        },
    },
    tardigrade::systems::{
        assign_tardigrade_animation, check_idle_tardigrade, despawn_dead_tardigrade,
    },
};
use crate::systems::movement::PositionSyncSystems;
use activable::{Activable, ActivableAppExt};
use bevy::prelude::*;
use composed::{
    CompositionAtlasAsset, CompositionAtlasLoader, apply_composed_enemy_visuals,
    ensure_composed_enemy_parts, prepare_composed_atlas_assets, update_composed_enemy_visuals,
};

/// Registers shared enemy behaviour systems and species handlers.
#[derive(Activable)]
pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<CompositionAtlasAsset>()
            .register_asset_loader(CompositionAtlasLoader);
        app.add_active_systems_in::<EnemyPlugin, _>(PostUpdate, apply_composed_enemy_visuals);
        app.add_active_systems::<EnemyPlugin, _>(
            // Behaviour/animation updates only run while the enemy subsystem is active.
            (
                check_no_behavior,
                on_enemy_depth_changed,
                on_composed_enemy_depth_changed,
                tick_enemy_behavior_timer,
                (
                    // Tied components - cleanup when behaviors end
                    tied_components_enemy_current_behavior_circle_around,
                    cleanup_orphaned_tween_children,
                ),
                (
                    // Mosquito
                    // Resolve transient attack state before selecting visuals so
                    // newly spawned idle enemies render idle until a real shot
                    // has both triggered and not yet cleared.
                    (clear_finished_mosquito_attacks, check_idle_mosquito)
                        .chain()
                        .after(check_no_behavior),
                    assign_mosquito_animation.after(check_idle_mosquito),
                    despawn_dead_mosquitoes,
                ),
                (
                    // Mosquiton
                    assign_mosquiton_animation.after(check_idle_mosquito),
                    apply_mosquiton_falling_physics,
                    (
                        prepare_composed_atlas_assets,
                        ensure_composed_enemy_parts,
                        update_composed_enemy_visuals,
                        detect_part_breakage,
                        trigger_mosquiton_authored_attack_cues,
                    )
                        .chain()
                        .after(PositionSyncSystems),
                    despawn_dead_mosquitons,
                    update_mosquiton_death_effect,
                ),
                (
                    // Spidey
                    // Composed pipeline (prepare/ensure/update) is registered
                    // in the Mosquiton block and operates generically on all
                    // entities with ComposedEnemyVisual.
                    assign_spidey_animation.after(check_no_behavior),
                    despawn_dead_spideys,
                    update_spidey_death_effect,
                ),
                (
                    // Tardigrade
                    assign_tardigrade_animation,
                    check_idle_tardigrade,
                    despawn_dead_tardigrade,
                ), // update_enemy_placeholder_direction,
                   // placeholder_tick_enemy_spawn_timer,
                   // placeholder_spawn_enemies_over_time,
            ),
        );
    }
}
