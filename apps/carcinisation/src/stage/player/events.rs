use bevy::prelude::*;

#[derive(Event, Message)]
pub struct CameraShakeTrigger;
// TODO camera shake can have some parameters based on the attack

#[derive(Event, Message)]
pub struct PlayerStartupTrigger;

#[derive(Event, Message)]
pub struct PlayerShutdownTrigger;
