pub mod destructibles;

use super::components::DestructibleType;
use crate::{
    pixel::CxAnimationBundle,
    stage::{
        components::{interactive::ColliderData, placement::Depth},
        data::ContainerSpawn,
    },
};
use bevy::prelude::*;
use carapace::prelude::{
    CxAnchor, CxAnimationDirection, CxAnimationDuration, CxAnimationFinishBehavior,
    CxFrameTransition,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Reflect, Serialize)]
pub struct DestructibleSpawn {
    #[reflect(ignore)]
    #[serde(default)]
    pub contains: Option<Box<ContainerSpawn>>,
    pub coordinates: Vec2,
    pub depth: Depth,
    pub destructible_type: DestructibleType,
    pub health: u32,
    /// Visible depths with hand-made visuals. When `None`, defaults to just
    /// the spawn's own `depth`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authored_depths: Option<Vec<Depth>>,
}

pub enum LampDepth {
    Three,
}

impl LampDepth {
    #[must_use]
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
    #[must_use]
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
    #[must_use]
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
    #[must_use]
    pub fn to_depth(&self) -> Depth {
        match self {
            CrystalDepth::Five => Depth::Five,
        }
    }
}

impl DestructibleSpawn {
    #[must_use]
    pub fn get_name(&self) -> Name {
        Name::new(self.show_type())
    }
    // TODO could use a Spawn trait
    #[must_use]
    pub fn show_type(&self) -> String {
        format!("Destructible<{:?}>", self.destructible_type)
    }

    #[must_use]
    pub fn with_coordinates(mut self, value: Vec2) -> Self {
        self.coordinates = value;
        self
    }

    #[must_use]
    pub fn with_contains(mut self, value: Option<Box<ContainerSpawn>>) -> Self {
        self.contains = value;
        self
    }

    #[must_use]
    pub fn with_depth(mut self, value: Depth) -> Self {
        self.depth = value;
        self
    }

    #[must_use]
    pub fn with_health(mut self, value: u32) -> Self {
        self.health = value;
        self
    }

    #[must_use]
    pub fn drops(mut self, value: ContainerSpawn) -> Self {
        self.contains = Some(Box::new(value));
        self
    }

    #[must_use]
    pub fn lamp_base(x: f32, y: f32, depth: LampDepth) -> Self {
        Self {
            contains: None,
            coordinates: Vec2::new(x, y),
            destructible_type: DestructibleType::Lamp,
            health: 60,
            depth: depth.to_depth(),
            authored_depths: None,
        }
    }

    /**
     * depth needs to be 1 or 4
     */
    #[must_use]
    pub fn trashcan_base(x: f32, y: f32, depth: TrashcanDepth) -> Self {
        Self {
            destructible_type: DestructibleType::Trashcan,
            coordinates: Vec2::new(x, y),
            contains: None,
            health: 100,
            depth: depth.to_depth(),
            authored_depths: None,
        }
    }

    #[must_use]
    pub fn crystal_base(x: f32, y: f32, depth: CrystalDepth) -> Self {
        Self {
            destructible_type: DestructibleType::Crystal,
            coordinates: Vec2::new(x, y),
            contains: None,
            health: 300,
            depth: depth.to_depth(),
            authored_depths: None,
        }
    }

    #[must_use]
    pub fn mushroom_base(x: f32, y: f32, depth: MushroomDepth) -> Self {
        Self {
            destructible_type: DestructibleType::Mushroom,
            coordinates: Vec2::new(x, y),
            contains: None,
            health: 120,
            depth: depth.to_depth(),
            authored_depths: None,
        }
    }
}

pub struct AnimationData {
    pub anchor: CxAnchor,
    pub collider_data: ColliderData,
    pub direction: CxAnimationDirection,
    pub finish_behavior: CxAnimationFinishBehavior,
    pub frames: usize,
    pub speed: u64,
    pub sprite_path: String,
}

impl AnimationData {
    #[must_use]
    pub fn make_animation_bundle(&self) -> CxAnimationBundle {
        CxAnimationBundle::from_parts(
            self.direction,
            CxAnimationDuration::millis_per_animation(self.speed),
            self.finish_behavior,
            CxFrameTransition::default(),
        )
    }
}

impl Default for AnimationData {
    fn default() -> Self {
        AnimationData {
            anchor: CxAnchor::BottomCenter,
            collider_data: ColliderData::new(),
            direction: CxAnimationDirection::Forward,
            finish_behavior: CxAnimationFinishBehavior::Mark,
            frames: 0,
            speed: 0,
            sprite_path: String::new(),
        }
    }
}
