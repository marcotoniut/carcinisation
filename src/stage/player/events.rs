use bevy::prelude::*;

#[derive(Event)]
pub struct CameraShakeTrigger;

#[derive(Event)]
pub struct PlayerStartupTrigger;

#[derive(Event)]
pub struct PlayerShutdownTrigger;
