use bevy::prelude::*;

#[derive(Component)]
pub struct Object;

// TODO impl more complex collision algorithm
#[derive(Clone, Component, Debug)]
pub enum Collision {
    Box(Vec2),
    Circle(f32),
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
