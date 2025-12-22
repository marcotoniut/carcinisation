use std::time::Duration;

use bevy::{
    math::{Rect, Vec2},
    prelude::Name,
};
use carcinisation::stage::{
    data::{EnemySpawn, ObjectSpawn, PickupSpawn, StageData, StageSpawn, StageStep},
    destructible::data::DestructibleSpawn,
};

use crate::builders::thumbnail::*;
use crate::timeline::{
    cinematic_duration, stop_duration, tween_travel_duration, StageTimelineConfig,
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
        let mut current_position = self.start_coordinates;
        let mut current_elapsed: Duration = Duration::ZERO;
        let config = StageTimelineConfig::SLIDER;

        for step in &self.steps {
            match step {
                StageStep::Tween(s) => {
                    let time_to_move = tween_travel_duration(current_position, s);

                    if current_elapsed + time_to_move > elapsed {
                        let t =
                            (elapsed - current_elapsed).as_secs_f32() / time_to_move.as_secs_f32();
                        return current_position.lerp(s.coordinates, t);
                    }

                    current_position = s.coordinates;
                    current_elapsed += time_to_move;
                }
                StageStep::Stop(s) => {
                    let duration = stop_duration(s, config);
                    if current_elapsed + duration > elapsed {
                        return current_position;
                    }
                    current_elapsed += duration;
                }
                StageStep::Cinematic(s) => {
                    let duration = cinematic_duration(s, config);
                    if current_elapsed + duration > elapsed {
                        return current_position;
                    }
                    current_elapsed += duration;
                }
            }
        }
        current_position
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
    /// Returns the thumbnail sprite and optional texture rect.
    fn get_thumbnail(&self) -> (String, Option<Rect>);
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

    fn get_thumbnail(&self) -> (String, Option<Rect>) {
        match self {
            StageSpawn::Destructible(DestructibleSpawn {
                destructible_type, ..
            }) => get_destructible_thumbnail(*destructible_type, self.get_depth()),
            StageSpawn::Enemy(EnemySpawn { enemy_type, .. }) => {
                get_enemy_thumbnail(*enemy_type, self.get_depth())
            }
            StageSpawn::Object(ObjectSpawn { object_type, .. }) => {
                get_object_thumbnail(*object_type, self.get_depth())
            }
            StageSpawn::Pickup(PickupSpawn { pickup_type, .. }) => {
                get_pickup_thumbnail(*pickup_type, self.get_depth())
            }
        }
    }
}
