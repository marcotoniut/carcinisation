use bevy::prelude::*;

use super::{components::Depth, data::StageSpawn};

#[derive(Event)]
pub struct StageStepTrigger {}

#[derive(Event)]
pub struct StageSpawnTrigger {
    pub spawn: StageSpawn,
}

#[derive(Event)]
pub struct DepthChanged {
    pub entity: Entity,
    pub depth: Depth,
}
