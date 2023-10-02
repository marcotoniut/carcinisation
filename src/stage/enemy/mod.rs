pub mod bundles;
pub mod components;
pub mod data;
pub mod systems;

use bevy::prelude::*;

use self::systems::{behaviors::*, mosquito::*, tardigrade::*, *};
use super::{GameState, StageState};
use crate::AppState;

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                check_no_behavior,
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
                .run_if(in_state(StageState::Running))
                .run_if(in_state(GameState::Running))
                .run_if(in_state(AppState::Game)),
        );
    }
}
