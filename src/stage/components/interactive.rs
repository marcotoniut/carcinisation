use bevy::prelude::*;

#[derive(Component)]
pub struct Object;

#[derive(Clone, Debug)]
pub enum Collision {
    Box(Vec2),
    Circle(f32),
}

// TODO impl more complex collision algorithm
#[derive(Clone, Component, Debug)]
pub struct CollisionData {
    pub collision: Collision,
    pub offset: Vec2,
}

impl CollisionData {
    pub fn new(collision: Collision) -> Self {
        Self {
            collision,
            offset: Vec2::ZERO,
        }
    }

    pub fn with_offset(mut self, offset: Vec2) -> Self {
        self.offset = offset;
        self
    }
}

#[derive(Clone, Component, Debug)]
pub struct Flickerer;

// Should hittable specify whether you can hit with Melee, ranged or both?
#[derive(Clone, Component, Debug)]
pub struct Hittable;

// TODO? critical kill
#[derive(Clone, Component, Debug)]
pub struct Dead;

#[derive(Clone, Component, Debug)]
pub struct Health(pub u32);
