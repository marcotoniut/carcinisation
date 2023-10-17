use std::{rc::Rc, sync::Arc};

use bevy::prelude::*;

use super::{
    components::placement::Depth,
    data::{StageData, StageSpawn},
};

#[derive(Event)]
pub struct StageRestart {}

#[derive(Event)]
pub struct NextStepEvent {}

#[derive(Event)]
pub struct StageClearedEvent {}

#[derive(Event)]
pub struct StageGameOverEvent {}

#[derive(Event)]
pub struct StageSpawnEvent {
    pub spawn: StageSpawn,
}

#[derive(Event)]
pub struct StageStartupEvent {
    pub data: Arc<StageData>,
}

#[derive(Event)]
pub struct DepthChangedEvent {
    pub entity: Entity,
    pub depth: Depth,
}

impl DepthChangedEvent {
    pub fn new(entity: Entity, depth: Depth) -> Self {
        DepthChangedEvent { entity, depth }
    }
}

#[derive(Event)]
pub struct DamageEvent {
    pub entity: Entity,
    pub value: u32,
}

impl DamageEvent {
    pub fn new(entity: Entity, value: u32) -> Self {
        DamageEvent { entity, value }
    }
}
