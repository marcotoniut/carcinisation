use std::time::Duration;

use bevy::{
    core::Name,
    math::{Rect, Vec2},
};
use carcinisation::stage::{
    data::{EnemySpawn, ObjectSpawn, PickupSpawn, StageData, StageSpawn, StageStep},
    destructible::data::DestructibleSpawn,
};

use crate::builders::thumbnail::*;

pub trait StageDataUtils {
    fn calculate_camera_position(&self, elapsed: Duration) -> Vec2;
    fn dynamic_spawn_count(&self) -> usize;
}

impl StageDataUtils for StageData {
    fn calculate_camera_position(&self, elapsed: Duration) -> Vec2 {
        let mut current_position = self.start_coordinates.unwrap_or(Vec2::ZERO);
        let mut current_elapsed: Duration = Duration::ZERO;

        for step in &self.steps {
            match step {
                StageStep::Movement(s) => {
                    let distance = s.coordinates.distance(current_position);
                    let time_to_move = Duration::from_secs_f32(distance / s.base_speed);

                    if current_elapsed + time_to_move > elapsed {
                        let t =
                            (elapsed - current_elapsed).as_secs_f32() / time_to_move.as_secs_f32();
                        return current_position.lerp(s.coordinates, t);
                    }

                    current_position = s.coordinates;
                    current_elapsed += time_to_move;
                }
                StageStep::Stop(s) => {
                    current_elapsed += s.max_duration.unwrap_or(Duration::ZERO);

                    if current_elapsed > elapsed {
                        return current_position;
                    }
                }
                StageStep::Cinematic(_) => {
                    // Handle Cinematic step if necessary
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
                StageStep::Movement(s) => s.spawns.len(),
                StageStep::Stop(x) => x.spawns.len(),
            })
            .sum::<usize>()
    }
}

pub trait StageSpawnUtils {
    fn get_editor_name_component(&self, index: usize) -> Name;
    fn get_depth_editor_z_index(&self) -> f32;
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

    // TODO this should be implemented only in the editor
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
            }) => get_destructible_thumbnail(destructible_type.clone(), self.get_depth()),
            StageSpawn::Enemy(EnemySpawn { enemy_type, .. }) => {
                get_enemy_thumbnail(enemy_type.clone(), self.get_depth())
            }
            StageSpawn::Object(ObjectSpawn { object_type, .. }) => {
                get_object_thumbnail(object_type.clone(), self.get_depth())
            }
            StageSpawn::Pickup(PickupSpawn { pickup_type, .. }) => {
                get_pickup_thumbnail(pickup_type.clone(), self.get_depth())
            }
        }
    }
}
