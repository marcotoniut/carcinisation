use bevy::prelude::*;

use super::{components::Depth, data::StageSpawn};

#[derive(Event)]
pub struct StageRestart {}

#[derive(Event)]
pub struct StageStepTrigger {}

#[derive(Event)]
pub struct StageClearedTrigger {}

#[derive(Event)]
pub struct StageGameOverTrigger {}

#[derive(Event)]
pub struct StageSpawnTrigger {
    pub spawn: StageSpawn,
}

#[derive(Event)]
pub struct DepthChanged {
    pub entity: Entity,
    pub depth: Depth,
}
