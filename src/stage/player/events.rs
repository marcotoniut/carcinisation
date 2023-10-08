use bevy::prelude::*;

#[derive(Event)]
pub struct CameraShakeEvent;

#[derive(Event)]
pub struct PlayerStartupEvent;

#[derive(Event)]
pub struct PlayerShutdownEvent;
