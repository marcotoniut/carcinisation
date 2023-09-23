use bevy::prelude::*;

use super::data::ContainerSpawn;

#[derive(Component)]
pub struct Stage {}

#[derive(Component)]
pub struct Destructible {}

#[derive(Component)]
pub struct Object {}

// TODO should go in UI
#[derive(Component, Debug, Clone)]
pub struct StageClearedText {}

#[derive(Component, Debug, Clone)]
pub enum Collision {
    Box(Vec2),
    Circle(f32),
}

// TODO impl more complex collision algorithm

#[derive(Component, Debug, Clone)]
pub struct Health(pub u32);

#[derive(Component, Debug, Clone)]
pub struct Hittable {}

// TODO? critical kill
#[derive(Component, Debug, Clone)]
pub struct Dead;

#[derive(Component, Debug, Clone)]
pub struct SpawnDrop {
    pub contains: ContainerSpawn,
    pub entity: Entity,
}
