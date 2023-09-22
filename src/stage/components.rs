use bevy::prelude::*;

use super::{data::EnemyType, enemy::components::EnemyInstance};

#[derive(Component)]
pub struct Stage {}

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

// TODO? critical kill
#[derive(Component, Debug, Clone)]
pub struct Dead;

#[derive(Component, Debug, Clone)]
pub enum Drop {
    // Pickup(Pickup),
    Enemy(EnemyInstance),
}
