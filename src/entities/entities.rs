use bevy::prelude::*;

pub enum SpawnableTypes {
    RANGED,
    MELEE,
    BOTH
}

#[derive(Component)]
pub struct SpawnableEntity {
    pub asset_path: String,
    pub animation_frames: i8,
    pub health: i8,
    pub damage: i8
}

#[derive(Component)]
pub struct SpawnableType {
    pub class: SpawnableTypes
}

#[derive(Bundle)]
pub struct SpawnableEntityBundle{
    entity: SpawnableEntity,
    entity_type: SpawnableType
}