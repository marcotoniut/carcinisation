use std::time::Duration;

use bevy::{math::Vec2, prelude::Name};
use carcinisation_ors::stage::{
    data::{EnemySpawn, ObjectSpawn, PickupSpawn, StageData, StageSpawn, StageStep},
    destructible::data::DestructibleSpawn,
    projection::walk_steps_at_elapsed,
};

/// Editor helpers derived from stage data.
pub trait StageDataUtils {
    /// Returns the camera position at a given elapsed time.
    fn calculate_camera_position(&self, elapsed: Duration) -> Vec2;
    /// Counts dynamic spawns across stage steps.
    fn dynamic_spawn_count(&self) -> usize;
}

impl StageDataUtils for StageData {
    fn calculate_camera_position(&self, elapsed: Duration) -> Vec2 {
        walk_steps_at_elapsed(self, elapsed).camera_position
    }

    fn dynamic_spawn_count(&self) -> usize {
        self.steps
            .iter()
            .map(|x| match x {
                StageStep::Cinematic(_) => 0,
                StageStep::Tween(s) => s.spawns.len(),
                StageStep::Stop(x) => x.spawns.len(),
            })
            .sum::<usize>()
    }
}

/// Editor helpers for stage spawn visualization.
pub trait StageSpawnUtils {
    /// Returns a display name including depth and index.
    fn get_editor_name_component(&self, index: usize) -> Name;
    /// Returns a z-index for layering in the editor view.
    fn get_depth_editor_z_index(&self) -> f32;
}

impl StageSpawnUtils for StageSpawn {
    fn get_editor_name_component(&self, index: usize) -> Name {
        Name::new(format!(
            "SG {} {} ({})",
            index,
            self.show_type(),
            self.get_depth().to_i8(),
        ))
    }

    fn get_depth_editor_z_index(&self) -> f32 {
        match self {
            StageSpawn::Destructible(DestructibleSpawn { depth, .. }) => {
                10.0 - depth.to_f32() + 0.2
            }
            StageSpawn::Enemy(EnemySpawn { depth, .. }) => -depth.to_f32() + 0.4,
            StageSpawn::Object(ObjectSpawn { depth, .. }) => -depth.to_f32() + 0.3,
            StageSpawn::Pickup(PickupSpawn { depth, .. }) => -depth.to_f32() + 0.1,
        }
    }
}
