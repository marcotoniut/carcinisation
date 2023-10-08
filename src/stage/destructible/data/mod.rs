pub mod destructibles;

use bevy::prelude::*;

use seldom_pixel::prelude::{
    PxAnchor, PxAnimationBundle, PxAnimationDirection, PxAnimationDuration,
    PxAnimationFinishBehavior,
};

use crate::stage::{components::interactive::Collision, data::ContainerSpawn};

use super::components::DestructibleType;

#[derive(Clone, Debug)]
pub struct DestructibleSpawn {
    pub contains: Option<Box<ContainerSpawn>>,
    pub coordinates: Vec2,
    pub depth: u8,
    pub destructible_type: DestructibleType,
    pub health: u32,
}

impl DestructibleSpawn {
    pub fn with_coordinates(mut self, value: Vec2) -> Self {
        self.coordinates = value;
        self
    }

    pub fn with_contains(mut self, value: Option<Box<ContainerSpawn>>) -> Self {
        self.contains = value;
        self
    }

    pub fn with_depth(mut self, value: u8) -> Self {
        self.depth = value;
        self
    }

    pub fn with_health(mut self, value: u32) -> Self {
        self.health = value;
        self
    }

    pub fn drops(mut self, value: ContainerSpawn) -> Self {
        self.contains = Some(Box::new(value));
        self
    }

    pub fn lamp_base(x: f32, y: f32) -> Self {
        Self {
            contains: None,
            coordinates: Vec2::new(x, y),
            destructible_type: DestructibleType::Lamp,
            health: 60,
            depth: 5,
        }
    }

    /**
     * depth needs to be 1 or 4
     */
    pub fn trashcan_base(x: f32, y: f32, depth: u8) -> Self {
        Self {
            destructible_type: DestructibleType::Trashcan,
            coordinates: Vec2::new(x, y),
            contains: None,
            health: 100,
            depth,
        }
    }

    pub fn crystal_base(x: f32, y: f32) -> Self {
        Self {
            destructible_type: DestructibleType::Crystal,
            coordinates: Vec2::new(x, y),
            contains: None,
            health: 300,
            depth: 4,
        }
    }

    pub fn mushroom_base(x: f32, y: f32) -> Self {
        Self {
            destructible_type: DestructibleType::Mushroom,
            coordinates: Vec2::new(x, y),
            contains: None,
            health: 120,
            depth: 5,
        }
    }
}

pub struct AnimationData {
    pub anchor: PxAnchor,
    pub collision: Collision,
    pub collision_offset: Vec2,
    pub direction: PxAnimationDirection,
    pub finish_behavior: PxAnimationFinishBehavior,
    pub frames: usize,
    pub speed: u64,
    pub sprite_path: String,
}

impl AnimationData {
    pub fn make_animation_bundle(&self) -> PxAnimationBundle {
        PxAnimationBundle {
            duration: PxAnimationDuration::millis_per_animation(self.speed),
            on_finish: self.finish_behavior,
            direction: self.direction,
            ..default()
        }
    }
}

impl Default for AnimationData {
    fn default() -> Self {
        AnimationData {
            anchor: PxAnchor::BottomCenter,
            collision: Collision::Box(Vec2::new(0., 0.)),
            collision_offset: Vec2::new(0., 0.),
            direction: PxAnimationDirection::Foreward,
            finish_behavior: PxAnimationFinishBehavior::Mark,
            frames: 0,
            speed: 0,
            sprite_path: String::from(""),
        }
    }
}
