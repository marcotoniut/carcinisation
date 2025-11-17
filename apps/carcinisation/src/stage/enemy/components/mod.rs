pub mod behavior;

use bevy::prelude::*;
use cween::structs::MovementDirection;

#[derive(Component, Debug, Default)]
pub struct Enemy;

#[derive(Component, Clone, Debug, Reflect)]
pub struct CircleAround {
    pub radius: f32,
    pub center: Vec2,
    pub time_offset: f32,
    pub direction: MovementDirection,
}

#[derive(Component, Clone, Debug, Reflect)]
pub struct LinearMovement {
    pub direction: Vec2,
    pub trayectory: f32,
    // TODO replace with LinearMovement2DReached
    pub reached_x: bool,
    pub reached_y: bool,
}

// Enemies

#[derive(Component)]
pub struct EnemySpidey;

// Bosses

#[derive(Component)]
pub struct EnemyMarauder;

#[derive(Component)]
pub struct EnemySpidomonsta {}

#[derive(Component)]
pub struct EnemyKyle {}
