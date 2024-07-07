use carcinisation::stage::{
    data::{EnemySpawn, ObjectSpawn, PickupSpawn, StageSpawn},
    destructible::data::DestructibleSpawn,
};

pub trait DepthEditorZIndex {
    fn get_depth_editor_z_index(&self) -> f32;
}

impl DepthEditorZIndex for StageSpawn {
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
}
