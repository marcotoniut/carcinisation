use std::time::Duration;

use bevy::prelude::*;

use crate::plugins::movement::structs::MovementVec2Position;

use super::data::ContainerSpawn;

#[derive(Component)]
pub struct Stage {}

#[derive(Component, Debug, Clone, Copy)]
pub struct Depth(pub usize);

#[derive(Component, Debug)]
pub struct InView {}

#[derive(Component)]
pub struct Destructible {}

#[derive(Component)]
pub struct Object {}

// TODO should go in UI
#[derive(Clone, Component, Debug)]
pub struct StageClearedText {}

#[derive(Clone, Component, Debug)]
pub enum Collision {
    Box(Vec2),
    Circle(f32),
}

#[derive(Clone, Component, Debug)]
pub struct Flickerer;

#[derive(Clone, Component, Debug)]
pub struct DamageFlicker {
    pub phase_start: Duration,
    pub count: usize,
}

#[derive(Component, Debug)]
pub struct InvertFilter;

// TODO impl more complex collision algorithm

#[derive(Clone, Component, Debug)]
pub struct Health(pub u32);

#[derive(Clone, Component, Debug)]
pub struct InflictsDamage(pub u32);

// Should hittable specify whether you can hit with Melee, ranged or both?
#[derive(Clone, Component, Debug)]
pub struct Hittable {}

// TODO? critical kill
#[derive(Clone, Component, Debug)]
pub struct Dead;

#[derive(Clone, Component, Debug)]
pub struct SpawnDrop {
    pub contains: ContainerSpawn,
    pub entity: Entity,
}

#[derive(Clone, Component, Debug)]
pub struct RailPosition(pub Vec2);

impl MovementVec2Position for RailPosition {
    fn get(&self) -> Vec2 {
        self.0
    }
    fn set(&mut self, position: Vec2) {
        self.0 = position;
    }
    fn add(&mut self, position: Vec2) {
        self.0 += position;
    }
}
