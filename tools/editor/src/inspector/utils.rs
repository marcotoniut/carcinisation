use bevy::{core::Name, math::Rect};
use carcinisation::stage::{
    data::{EnemySpawn, ObjectSpawn, PickupSpawn, StageSpawn},
    destructible::data::DestructibleSpawn,
};

use crate::builders::thumbnail::*;

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
