pub mod destructibles;

use super::components::DestructibleType;
use crate::{
    pixel::PxAnimationBundle,
    stage::{
        components::{interactive::ColliderData, placement::Depth},
        data::ContainerSpawn,
    },
};
use bevy::prelude::*;
use seldom_pixel::prelude::{
    PxAnchor, PxAnimationDirection, PxAnimationDuration, PxAnimationFinishBehavior,
    PxFrameTransition,
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "derive-ts")]
use ts_rs::TS;

#[cfg_attr(feature = "derive-ts", derive(TS))]
#[cfg_attr(feature = "derive-ts", ts(export))]
#[derive(Clone, Debug, Deserialize, Reflect, Serialize)]
pub struct DestructibleSpawn {
    #[reflect(ignore)]
    #[serde(default)]
    pub contains: Option<Box<ContainerSpawn>>,
    #[cfg_attr(feature = "derive-ts", ts(type = "[number, number]"))]
    pub coordinates: Vec2,
    pub depth: Depth,
    pub destructible_type: DestructibleType,
    pub health: u32,
}

pub enum LampDepth {
    Three,
}

impl LampDepth {
    pub fn to_depth(&self) -> Depth {
        match self {
            LampDepth::Three => Depth::Three,
        }
    }
}

// Can Depth implement isize?
pub enum TrashcanDepth {
    Six,
    Four,
}

impl TrashcanDepth {
    pub fn to_depth(&self) -> Depth {
        match self {
            TrashcanDepth::Six => Depth::Six,
            TrashcanDepth::Four => Depth::Four,
        }
    }
}

pub enum MushroomDepth {
    Four,
}

impl MushroomDepth {
    pub fn to_depth(&self) -> Depth {
        match self {
            MushroomDepth::Four => Depth::Four,
        }
    }
}

pub enum CrystalDepth {
    Five,
}

impl CrystalDepth {
    pub fn to_depth(&self) -> Depth {
        match self {
            CrystalDepth::Five => Depth::Five,
        }
    }
}

impl DestructibleSpawn {
    pub fn get_name(&self) -> Name {
        Name::new(self.show_type())
    }
    // TODO could use a Spawn trait
    pub fn show_type(&self) -> String {
        format!("Destructible<{:?}>", self.destructible_type)
    }

    pub fn with_coordinates(mut self, value: Vec2) -> Self {
        self.coordinates = value;
        self
    }

    pub fn with_contains(mut self, value: Option<Box<ContainerSpawn>>) -> Self {
        self.contains = value;
        self
    }

    pub fn with_depth(mut self, value: Depth) -> Self {
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

    pub fn lamp_base(x: f32, y: f32, depth: LampDepth) -> Self {
        Self {
            contains: None,
            coordinates: Vec2::new(x, y),
            destructible_type: DestructibleType::Lamp,
            health: 60,
            depth: depth.to_depth(),
        }
    }

    /**
     * depth needs to be 1 or 4
     */
    pub fn trashcan_base(x: f32, y: f32, depth: TrashcanDepth) -> Self {
        Self {
            destructible_type: DestructibleType::Trashcan,
            coordinates: Vec2::new(x, y),
            contains: None,
            health: 100,
            depth: depth.to_depth(),
        }
    }

    pub fn crystal_base(x: f32, y: f32, depth: CrystalDepth) -> Self {
        Self {
            destructible_type: DestructibleType::Crystal,
            coordinates: Vec2::new(x, y),
            contains: None,
            health: 300,
            depth: depth.to_depth(),
        }
    }

    pub fn mushroom_base(x: f32, y: f32, depth: MushroomDepth) -> Self {
        Self {
            destructible_type: DestructibleType::Mushroom,
            coordinates: Vec2::new(x, y),
            contains: None,
            health: 120,
            depth: depth.to_depth(),
        }
    }
}

pub struct AnimationData {
    pub anchor: PxAnchor,
    pub collider_data: ColliderData,
    pub direction: PxAnimationDirection,
    pub finish_behavior: PxAnimationFinishBehavior,
    pub frames: usize,
    pub speed: u64,
    pub sprite_path: String,
}

impl AnimationData {
    pub fn make_animation_bundle(&self) -> PxAnimationBundle {
        PxAnimationBundle::from_parts(
            self.direction,
            PxAnimationDuration::millis_per_animation(self.speed),
            self.finish_behavior,
            PxFrameTransition::default(),
        )
    }
}

impl Default for AnimationData {
    fn default() -> Self {
        AnimationData {
            anchor: PxAnchor::BottomCenter,
            collider_data: ColliderData::new(),
            direction: PxAnimationDirection::Foreward,
            finish_behavior: PxAnimationFinishBehavior::Mark,
            frames: 0,
            speed: 0,
            sprite_path: String::from(""),
        }
    }
}
