use bevy::prelude::*;

#[derive(Component)]
pub struct Object;

#[derive(Clone, Copy, Debug, Reflect)]
pub enum CollisionShape {
    Box(Vec2),
    Circle(f32),
}

impl CollisionShape {
    pub fn point_collides(&self, collision_position: Vec2, point_position: Vec2) -> bool {
        let distance = collision_position.distance(point_position);
        match &self {
            CollisionShape::Box(size) => distance < size.x && distance < size.y,
            CollisionShape::Circle(radius) => distance <= *radius,
        }
    }
}

#[derive(Clone, Copy, Debug, Reflect)]
pub struct Collision {
    pub shape: CollisionShape,
    pub defense: f32,
    pub offset: Vec2,
}

impl Collision {
    pub fn new(collision: CollisionShape) -> Self {
        Self {
            shape: collision,
            defense: 1.,
            offset: Vec2::ZERO,
        }
    }

    pub fn new_circle(radius: f32) -> Self {
        Self::new(CollisionShape::Circle(radius))
    }

    pub fn new_box(size: Vec2) -> Self {
        Self::new(CollisionShape::Box(size))
    }

    pub fn new_scaled(mut self, scale: f32) -> Self {
        let mut new = self.clone();
        match new.shape {
            CollisionShape::Box(ref mut size) => {
                size.x *= scale;
                size.y *= scale;
            }
            CollisionShape::Circle(ref mut radius) => {
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

impl From<CollisionShape> for Collision {
    fn from(collision: CollisionShape) -> Self {
        Collision::new(collision)
    }
}

#[derive(Clone, Component, Debug, Reflect)]
pub struct CollisionData(pub Vec<Collision>);

impl CollisionData {
    pub fn new() -> Self {
        Self(vec![])
    }

    pub fn from_one(collision: Collision) -> Self {
        Self(vec![collision])
    }

    pub fn from_many(collisions: Vec<Collision>) -> Self {
        Self(collisions)
    }

    pub fn point_collides_with(
        &self,
        collision_position: Vec2,
        point_position: Vec2,
    ) -> Vec<Collision> {
        self.0
            .iter()
            .filter(|collision| {
                collision
                    .shape
                    .point_collides(collision_position + collision.offset, point_position)
            })
            .cloned()
            .collect()
    }

    pub fn point_collides(
        &self,
        collision_position: Vec2,
        circle_position: Vec2,
    ) -> Option<&Collision> {
        self.0.iter().find(|collision| {
            collision
                .shape
                .point_collides(collision_position + collision.offset, circle_position)
        })
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

#[derive(Clone, Component, Debug, Reflect)]
pub struct Health(pub u32);
