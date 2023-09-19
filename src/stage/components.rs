use bevy::prelude::*;

#[derive(Component)]
pub struct Stage {}

// TODO should go in UI
#[derive(Component)]
pub struct StageClearedText {}

// TODO should go in UI
#[derive(Component)]
pub enum Collision {
    Box(Vec2),
    Circle(f32),
}
