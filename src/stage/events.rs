use bevy::prelude::*;

use super::data::StageSpawn;

#[derive(Event)]
pub struct StageStepTrigger {}

#[derive(Event)]
pub struct StageSpawnTrigger {
    pub spawn: StageSpawn,
}
