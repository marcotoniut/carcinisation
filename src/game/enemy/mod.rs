use bevy::prelude::*;

use self::{resources::*, systems::*};

pub mod components;
pub mod resources;
pub mod systems;

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EnemySpawnTimer>()
            .add_systems(Startup, spawn_enemies)
            .add_systems(Update, (enemy_movement, confine_enemy_movement).chain())
            .add_systems(Update, update_enemy_direction)
            .add_systems(Update, tick_enemy_spawn_timer)
            .add_systems(Update, spawn_enemies_over_time);
    }
}
