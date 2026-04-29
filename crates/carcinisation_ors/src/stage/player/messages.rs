use bevy::prelude::*;

#[derive(Event, Message)]
pub struct CameraShakeEvent;
// TODO camera shake can have some parameters based on the attack

#[derive(Event, Message)]
pub struct PlayerStartupEvent;

#[derive(Event, Message)]
pub struct PlayerShutdownEvent;
