use bevy::prelude::*;

#[derive(Event)]
pub struct CameraShakeTrigger;
// TODO camera shake can have some parameters based on the attack

#[derive(Event)]
pub struct PlayerStartupTrigger;

#[derive(Event)]
pub struct PlayerShutdownTrigger;
