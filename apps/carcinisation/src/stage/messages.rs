//! Message types emitted during stage progression, spawns, and damage handling.

use std::sync::Arc;

use asset_pipeline::aseprite::{AnimationEventKind, Point};
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

#[derive(new, Message)]
/// Damage targeted at a composed semantic part rather than the whole entity.
pub struct PartDamageMessage {
    pub entity: Entity,
    pub part_id: String,
    pub value: u32,
}

#[derive(Clone, Debug, Message)]
/// One-shot authored cue emitted by composed animation playback when a frame is entered.
pub struct ComposedAnimationCueMessage {
    pub entity: Entity,
    pub tag: String,
    pub frame_index: usize,
    pub source_frame: usize,
    pub kind: AnimationEventKind,
    pub id: String,
    pub part_id: Option<String>,
    pub local_offset: Point,
}
