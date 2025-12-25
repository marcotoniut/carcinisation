use bevy::prelude::*;
use derive_more::From;
use derive_new::new;

#[derive(Clone, Copy, Debug, Reflect)]
pub enum ColliderShape {
    Box(Vec2),
    Circle(f32),
}

impl ColliderShape {
    pub fn point_collides(&self, collider_position: Vec2, point_position: Vec2) -> bool {
        match &self {
            ColliderShape::Box(size) => {
                let delta = (point_position - collider_position).abs();
                delta.x <= size.x && delta.y <= size.y
            }
            ColliderShape::Circle(radius) => {
                collider_position.distance_squared(point_position) <= *radius * *radius
            }
        }
    }

    pub fn overlaps(
        &self,
        self_position: Vec2,
        other: &ColliderShape,
        other_position: Vec2,
    ) -> bool {
        match (self, other) {
            (ColliderShape::Circle(a), ColliderShape::Circle(b)) => {
                let radius = a + b;
                self_position.distance_squared(other_position) <= radius * radius
            }
            (ColliderShape::Box(a), ColliderShape::Box(b)) => {
                let delta = (self_position - other_position).abs();
                delta.x <= a.x + b.x && delta.y <= a.y + b.y
            }
            (ColliderShape::Box(half), ColliderShape::Circle(radius)) => {
                box_overlaps_circle(*half, self_position, *radius, other_position)
            }
            (ColliderShape::Circle(radius), ColliderShape::Box(half)) => {
                box_overlaps_circle(*half, other_position, *radius, self_position)
            }
        }
    }
}

fn box_overlaps_circle(half: Vec2, box_center: Vec2, radius: f32, circle_center: Vec2) -> bool {
    let min = box_center - half;
    let max = box_center + half;
    let closest = circle_center.clamp(min, max);
    circle_center.distance_squared(closest) <= radius * radius
}

#[derive(new, Clone, Copy, Debug, From, Reflect)]
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
        let mut new = self;
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
        point_position: Vec2,
    ) -> Option<&Collider> {
        self.0.iter().find(|x| {
            x.shape
                .point_collides(collider_position + x.offset, point_position)
        })
    }
}

impl Default for ColliderData {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn box_overlaps_box() {
        let box_a = ColliderShape::Box(Vec2::new(2.0, 1.5));
        let box_b = ColliderShape::Box(Vec2::new(1.0, 1.0));

        assert!(box_a.overlaps(Vec2::ZERO, &box_b, Vec2::new(2.5, 0.0)));
        assert!(!box_a.overlaps(Vec2::ZERO, &box_b, Vec2::new(4.1, 0.0)));
    }

    #[test]
    fn circle_overlaps_circle() {
        let circle_a = ColliderShape::Circle(1.5);
        let circle_b = ColliderShape::Circle(1.0);

        assert!(circle_a.overlaps(Vec2::ZERO, &circle_b, Vec2::new(2.4, 0.0)));
        assert!(!circle_a.overlaps(Vec2::ZERO, &circle_b, Vec2::new(2.6, 0.0)));
    }

    #[test]
    fn box_overlaps_circle() {
        let box_a = ColliderShape::Box(Vec2::new(1.0, 1.0));
        let circle_b = ColliderShape::Circle(0.8);

        assert!(box_a.overlaps(Vec2::ZERO, &circle_b, Vec2::new(1.6, 0.0)));
        assert!(!box_a.overlaps(Vec2::ZERO, &circle_b, Vec2::new(2.2, 0.0)));
    }

    #[test]
    fn box_point_collides() {
        let collider = ColliderShape::Box(Vec2::new(2.0, 1.0));

        assert!(collider.point_collides(Vec2::ZERO, Vec2::new(1.9, 0.9)));
        assert!(!collider.point_collides(Vec2::ZERO, Vec2::new(2.1, 1.1)));
    }

    #[test]
    fn circle_point_collides() {
        let collider = ColliderShape::Circle(1.0);

        assert!(collider.point_collides(Vec2::ZERO, Vec2::new(0.7, 0.7)));
        assert!(!collider.point_collides(Vec2::ZERO, Vec2::new(0.9, 0.9)));
    }
}
