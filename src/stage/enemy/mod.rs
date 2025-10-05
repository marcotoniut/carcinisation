//! Enemy entity definitions, behaviours, and species-specific logic.

pub mod bundles;
pub mod components;
pub mod data;
pub mod entity;
pub mod mosquito;
mod systems;
pub mod tardigrade;

use self::{
    mosquito::systems::*,
    systems::{animation::on_enemy_depth_changed, behaviors::*},
    tardigrade::systems::*,
};
use bevy::prelude::*;

/// Registers shared enemy behaviour systems and species handlers.
pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<EnemyPluginUpdateState>().add_systems(
            Update,
            // Behaviour/animation updates only run while the enemy subsystem is active.
            (
                check_no_behavior,
                on_enemy_depth_changed,
                tick_enemy_behavior_timer,
                (
                    // Tied components
                    tied_components_enemy_current_behavior_circle_around
                ),
                (
                    // Mosquito
                    assign_mosquito_animation,
                    check_idle_mosquito,
                    despawn_dead_mosquitoes,
                ),
                (
                    // Tardigrade
                    assign_tardigrade_animation,
                    check_idle_tardigrade,
                    despawn_dead_tardigrade,
                ), // update_enemy_placeholder_direction,
                   // placeholder_tick_enemy_spawn_timer,
                   // placeholder_spawn_enemies_over_time,
            )
                .run_if(in_state(EnemyPluginUpdateState::Active)),
        );
    }
}

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
/// Stage-level flag controlling when enemy systems execute.
pub enum EnemyPluginUpdateState {
    #[default]
    Inactive,
    Active,
}
