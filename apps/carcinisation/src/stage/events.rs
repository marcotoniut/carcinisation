//! Event types emitted during stage progression, spawns, and damage handling.

use std::sync::Arc;

use bevy::prelude::*;
use derive_new::new;

use super::{
    components::placement::Depth,
    data::{StageData, StageSpawn},
};

#[derive(Message)]
/// Requests a full stage reset.
pub struct StageRestart;

#[derive(Event, Message)]
/// Signals that the scripted sequence should advance to the next step.
pub struct NextStepEvent;

#[derive(Clone, Event, Message)]
/// Triggered when the stage clears all objectives.
pub struct StageClearedTrigger;

#[derive(Event, Message)]
/// Indicates the player died during the current stage run.
pub struct StageDeathEvent;

#[derive(Event, Message)]
/// Requests spawning of a concrete `StageSpawn` instruction.
pub struct StageSpawnTrigger {
    pub spawn: StageSpawn,
}

#[derive(Event, Message)]
/// Fired when the stage first loads with the associated serialized data.
pub struct StageStartupTrigger {
    pub data: Arc<StageData>,
}

#[derive(new, Message)]
/// Broadcast when an entity moves between depth layers.
pub struct DepthChangedEvent {
    pub entity: Entity,
    pub depth: Depth,
}

#[derive(new, Message)]
/// Generic damage event consumed by combat systems.
pub struct DamageEvent {
    pub entity: Entity,
    pub value: u32,
}
