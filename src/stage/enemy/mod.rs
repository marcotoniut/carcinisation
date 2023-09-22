pub mod bundles;
pub mod components;
pub mod data;
pub mod resources;
pub mod systems;

use bevy::prelude::*;

use self::{
    resources::*,
    systems::{mosquito::despawn_dead_mosquitoes, *},
};
use super::{GameState, StageState};
use crate::AppState;

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EnemySpawnTimer>()
            .add_systems(OnEnter(AppState::Game), spawn_enemies)
            .add_systems(
                Update,
                (
                    (enemy_movement, confine_enemy_movement).chain(),
                    (check_enemy_got_hit, check_enemy_health).chain(),
                    despawn_dead_mosquitoes,
                    update_enemy_placeholder_direction,
                    placeholder_tick_enemy_spawn_timer,
                    placeholder_spawn_enemies_over_time,
                )
                    .run_if(in_state(AppState::Game))
                    .run_if(in_state(GameState::Running))
                    .run_if(in_state(StageState::Running)),
            )
            .add_systems(OnExit(AppState::Game), despawn_enemies);
    }
}
