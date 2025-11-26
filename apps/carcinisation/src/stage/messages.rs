//! Message types emitted during stage progression, spawns, and damage handling.

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
pub struct StageClearedEvent;

#[derive(Event, Message)]
/// Indicates the player died during the current stage run.
pub struct StageDeathEvent;

#[derive(Event, Message)]
/// Requests spawning of a concrete `StageSpawn` instruction.
pub struct StageSpawnEvent {
    pub spawn: StageSpawn,
}

#[derive(Event, Message)]
/// Fired when the stage first loads with the associated serialized data.
pub struct StageStartupEvent {
    pub data: Arc<StageData>,
}

#[derive(new, Message)]
/// Broadcast when an entity moves between depth layers.
pub struct DepthChangedMessage {
    pub entity: Entity,
    pub depth: Depth,
}

#[derive(new, Message)]
/// Generic damage message consumed by combat systems.
pub struct DamageMessage {
    pub entity: Entity,
    pub value: u32,
}
