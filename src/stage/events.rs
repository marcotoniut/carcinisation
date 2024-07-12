use std::sync::Arc;

use bevy::prelude::*;
use derive_new::new;

use super::{
    components::placement::Depth,
    data::{StageData, StageSpawn},
};

#[derive(Event)]
pub struct StageRestart;

#[derive(Event)]
pub struct NextStepEvent;

#[derive(Clone, Event)]
pub struct StageClearedTrigger;

#[derive(Event)]
pub struct StageDeathEvent;

#[derive(Event)]
pub struct StageSpawnTrigger {
    pub spawn: StageSpawn,
}

#[derive(Event)]
pub struct StageStartupTrigger {
    pub data: Arc<StageData>,
}

#[derive(new, Event)]
pub struct DepthChangedEvent {
    pub entity: Entity,
    pub depth: Depth,
}

#[derive(new, Event)]
pub struct DamageEvent {
    pub entity: Entity,
    pub value: u32,
}
