use bevy::prelude::*;
use derive_new::new;

#[derive(Component)]
pub struct Object;

#[derive(Clone, Copy, Debug, Reflect)]
pub enum ColliderShape {
    Box(Vec2),
    Circle(f32),
}

impl ColliderShape {
    pub fn point_collides(&self, collider_position: Vec2, point_position: Vec2) -> bool {
        let distance = collider_position.distance(point_position);
        match &self {
            ColliderShape::Box(size) => distance < size.x && distance < size.y,
            ColliderShape::Circle(radius) => distance <= *radius,
        }
    }
}

#[derive(new, Clone, Copy, Debug, Reflect)]
pub struct Collider {
    pub shape: ColliderShape,
    #[new(value = "1.")]
    pub defense: f32,
    #[new(default)]
    pub offset: Vec2,
}

impl Collider {
    pub fn new_circle(radius: f32) -> Self {
        Self::new(ColliderShape::Circle(radius))
    }

    pub fn new_box(size: Vec2) -> Self {
        Self::new(ColliderShape::Box(size))
    }

    pub fn new_scaled(self, scale: f32) -> Self {
        let mut new = self.clone();
        match new.shape {
            ColliderShape::Box(ref mut size) => {
                size.x *= scale;
                size.y *= scale;
            }
            ColliderShape::Circle(ref mut radius) => {
                *radius *= scale;
            }
        }
        new
    }

    pub fn with_defense(mut self, defense: f32) -> Self {
        self.defense = defense;
        self
    }

    pub fn with_offset(mut self, offset: Vec2) -> Self {
        self.offset = offset;
        self
    }
}

impl From<ColliderShape> for Collider {
    fn from(shape: ColliderShape) -> Self {
        Collider::new(shape)
    }
}

#[derive(Clone, Component, Debug, Reflect)]
pub struct ColliderData(pub Vec<Collider>);

impl ColliderData {
    pub fn new() -> Self {
        Self(vec![])
    }

    pub fn from_one(collider: Collider) -> Self {
        Self(vec![collider])
    }

    pub fn from_many(colliders: Vec<Collider>) -> Self {
        Self(colliders)
    }

    pub fn point_collides_with(
        &self,
        collider_position: Vec2,
        point_position: Vec2,
    ) -> Vec<Collider> {
        self.0
            .iter()
            .filter(|x| {
                x.shape
                    .point_collides(collider_position + x.offset, point_position)
            })
            .cloned()
            .collect()
    }

    pub fn point_collides(
        &self,
        collider_position: Vec2,
        circle_position: Vec2,
    ) -> Option<&Collider> {
        self.0.iter().find(|x| {
            x.shape
                .point_collides(collider_position + x.offset, circle_position)
        })
    }
}

#[derive(Clone, Component, Debug, Default)]
pub struct Flickerer;

// Should hittable specify whether you can hit with Melee, ranged or both?
#[derive(Clone, Component, Debug, Default)]
pub struct Hittable;

// TODO? critical kill
#[derive(Clone, Component, Debug, Default)]
pub struct Dead;

#[derive(Clone, Component, Debug, Reflect)]
pub struct Health(pub u32);
