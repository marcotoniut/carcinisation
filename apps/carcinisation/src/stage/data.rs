//! Serialized stage definitions: spawns, pickups, objects, and scripted steps.

use super::{
    components::{placement::Depth, CinematicStageStep, MovementStageStep, StopStageStep},
    destructible::data::DestructibleSpawn,
    enemy::{data::steps::EnemyStep, entity::EnemyType},
};
use crate::globals::{SCREEN_RESOLUTION, SCREEN_RESOLUTION_F32_H};
use bevy::{asset::Asset, prelude::*, reflect::Reflect};
use derive_more::From;
use derive_new::new;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationSecondsWithFrac};
use std::{collections::VecDeque, time::Duration};

lazy_static! {
    /// Convenience spawn position centred on the gameplay viewport.
    pub static ref DEFAULT_COORDINATES: Vec2 = SCREEN_RESOLUTION_F32_H.clone();
}

pub const GAME_BASE_SPEED: f32 = 15.0;

/// Common methods for spawn containers that may hold pickups or enemies.
pub trait Contains {
    fn with_contains(&mut self, value: Option<Box<ContainerSpawn>>);
    fn drops(&mut self, value: ContainerSpawn);
}

#[derive(Clone, Debug, Deserialize, Reflect, Serialize)]
/// Metadata required to load an animated skybox.
pub struct SkyboxData {
    pub path: String,
    pub frames: usize,
}

// deriving Default for simplicity's sake in defining the stage data
#[derive(Clone, Debug, Deserialize, Reflect, Serialize)]
/// Static props that can populate the stage background.
pub enum ObjectType {
    BenchBig,
    BenchSmall,
    Fibertree,
    RugparkSign,
}

#[derive(Clone, Debug, Deserialize, Reflect, Serialize)]
/// Pickup categories available for drop spawns.
pub enum PickupType {
    SmallHealthpack,
    BigHealthpack,
    // TODO
    // Weapon,
    // Ammo,
    // Shield,
}

#[derive(Clone, Debug, Deserialize, From, Reflect, Serialize)]
/// Container that either spawns a pickup or an enemy payload.
pub enum ContainerSpawn {
    Pickup(PickupDropSpawn),
    Enemy(EnemyDropSpawn),
}

// TODO move pickup data under its own module?
#[serde_as]
#[derive(Clone, Debug, Deserialize, Reflect, Serialize)]
/// Runtime spawn instruction for a pickup entity.
pub struct PickupSpawn {
    pub pickup_type: PickupType,
    pub coordinates: Vec2,
    #[serde(default)]
    #[serde_as(as = "DurationSecondsWithFrac")]
    pub elapsed: Duration,
    pub depth: Depth,
}

impl PickupSpawn {
    /// Builds a Bevy name for debugging.
    pub fn get_name(&self) -> Name {
        Name::new(self.show_type())
    }
    /// Display string describing the pickup type.
    pub fn show_type(&self) -> String {
        format!("Pickup<{:?}>", self.pickup_type)
    }
    pub fn with_elapsed_f32(mut self, value: f32) -> Self {
        self.elapsed = Duration::from_secs_f32(value);
        self
    }
    pub fn with_coordinates(mut self, value: Vec2) -> Self {
        self.coordinates = value;
        self
    }
    pub fn big_healthpack_base() -> Self {
        Self {
            pickup_type: PickupType::BigHealthpack,
            coordinates: Vec2::ZERO,
            elapsed: Duration::ZERO,
            depth: Depth::Six,
        }
    }
    pub fn small_healthpack_base() -> Self {
        Self {
            pickup_type: PickupType::SmallHealthpack,
            coordinates: Vec2::ZERO,
            elapsed: Duration::ZERO,
            depth: Depth::Six,
        }
    }
}

// TODO move pickup data under its own module?
#[derive(new, Clone, Debug, Deserialize, Reflect, Serialize)]
/// Template describing which pickup type should drop.
pub struct PickupDropSpawn {
    pub pickup_type: PickupType,
}

impl PickupDropSpawn {
    /// Creates a concrete spawn from this template at the provided location/depth.
    pub fn from_spawn(&self, coordinates: Vec2, depth: Depth) -> PickupSpawn {
        PickupSpawn {
            pickup_type: self.pickup_type.clone(),
            coordinates,
            depth,
            elapsed: Duration::ZERO,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Reflect, Serialize)]
/// Static prop placement within the stage map.
pub struct ObjectSpawn {
    pub object_type: ObjectType,
    pub coordinates: Vec2,
    pub depth: Depth,
}

impl ObjectSpawn {
    /// Builds a Bevy name for debugging.
    pub fn get_name(&self) -> Name {
        Name::new(self.show_type())
    }
    /// Display string describing the object type.
    pub fn show_type(&self) -> String {
        format!("Object<{:?}>", self.object_type)
    }

    pub fn with_coordinates(mut self, value: Vec2) -> Self {
        self.coordinates = value;
        self
    }

    pub fn bench_big_base(x: f32, y: f32) -> Self {
        Self {
            object_type: ObjectType::BenchBig,
            coordinates: Vec2::new(x, y),
            // TODO should be Six
            depth: Depth::Eight,
        }
    }

    pub fn bench_small_base(x: f32, y: f32) -> Self {
        Self {
            object_type: ObjectType::BenchSmall,
            coordinates: Vec2::new(x, y),
            // TODO should be Six
            depth: Depth::Eight,
        }
    }

    pub fn fibertree_base(x: f32, y: f32) -> Self {
        Self {
            object_type: ObjectType::Fibertree,
            coordinates: Vec2::new(x, y),
            depth: Depth::Two,
        }
    }

    pub fn rugpark_sign_base(x: f32, y: f32) -> Self {
        Self {
            object_type: ObjectType::RugparkSign,
            coordinates: Vec2::new(x, y),
            depth: Depth::Three,
        }
    }
}

#[serde_as]
#[derive(Clone, Debug, Default, Deserialize, Reflect, Serialize)]
pub struct EnemySpawn {
    pub enemy_type: EnemyType,
    #[serde(default)]
    #[serde_as(as = "DurationSecondsWithFrac")]
    pub elapsed: Duration,
    #[reflect(ignore)]
    #[serde(default)]
    pub contains: Option<Box<ContainerSpawn>>,
    pub coordinates: Vec2,
    pub speed: f32,
    #[serde(default)]
    pub steps: VecDeque<EnemyStep>,
    pub depth: Depth,
}

#[derive(Clone, Debug, Default, Deserialize, Reflect, Serialize)]
pub struct EnemyDropSpawn {
    pub enemy_type: EnemyType,
    #[reflect(ignore)]
    #[serde(default)]
    pub contains: Option<Box<ContainerSpawn>>,
    pub speed: f32,
    #[serde(default)]
    pub steps: VecDeque<EnemyStep>,
}

impl EnemyDropSpawn {
    pub fn from_spawn(&self, coordinates: Vec2, depth: Depth) -> EnemySpawn {
        EnemySpawn {
            enemy_type: self.enemy_type.clone(),
            coordinates,
            depth,
            speed: self.speed,
            steps: self.steps.clone(),
            contains: self.contains.clone(),
            elapsed: Duration::ZERO,
        }
    }
}

impl EnemySpawn {
    pub fn with_elapsed_f32(mut self, value: f32) -> Self {
        self.elapsed = Duration::from_secs_f32(value);
        self
    }
    pub fn with_coordinates(mut self, value: Vec2) -> Self {
        self.coordinates = value;
        self
    }
    pub fn with_x(mut self, value: f32) -> Self {
        self.coordinates.x = value;
        self
    }
    pub fn with_y(mut self, value: f32) -> Self {
        self.coordinates.y = value;
        self
    }
    pub fn with_speed(mut self, value: f32) -> Self {
        self.speed = value;
        self
    }
    pub fn with_steps(mut self, value: VecDeque<EnemyStep>) -> Self {
        self.steps = value;
        self
    }
    pub fn with_steps_vec(mut self, value: Vec<EnemyStep>) -> Self {
        self.steps = value.into();
        self
    }
    pub fn with_depth(mut self, value: Depth) -> Self {
        self.depth = value;
        self
    }
    /** TODO should I implement these as a trait Contains */
    pub fn with_contains(mut self, value: Option<Box<ContainerSpawn>>) -> Self {
        self.contains = value;
        self
    }
    pub fn drops(mut self, value: ContainerSpawn) -> Self {
        self.contains = Some(Box::new(value));
        self
    }

    pub fn add_step(mut self, value: EnemyStep) -> Self {
        self.steps.extend(vec![value]);
        self
    }

    // Enemies
    pub fn tardigrade_base() -> Self {
        Self {
            enemy_type: EnemyType::Tardigrade,
            coordinates: *DEFAULT_COORDINATES,
            depth: Depth::Six,
            elapsed: Duration::ZERO,
            speed: 0.5,
            steps: vec![].into(),
            contains: None,
        }
    }
    pub fn mosquito_base() -> Self {
        Self {
            enemy_type: EnemyType::Mosquito,
            coordinates: *DEFAULT_COORDINATES,
            depth: Depth::Five,
            elapsed: Duration::ZERO,
            speed: 2.0,
            steps: vec![].into(),
            contains: None,
        }
    }
    pub fn mosquito_variant_circle() -> Self {
        Self::mosquito_base().with_steps_vec(vec![EnemyStep::circle_around_base()
            .with_radius(12.)
            .into()])
    }
    pub fn mosquito_variant_approacher() -> Self {
        Self::mosquito_base()
            .with_depth(Depth::Eight)
            .with_x(SCREEN_RESOLUTION.x as f32 + 10.)
            .with_steps_vec(vec![
                EnemyStep::linear_movement_base()
                    .with_direction(-1., -0.1)
                    .with_trayectory(100.)
                    .depth_advance(1)
                    .into(),
                EnemyStep::linear_movement_base()
                    .depth_advance(2)
                    .with_direction(0.3, -0.2)
                    .with_trayectory(80.)
                    .into(),
                EnemyStep::idle_base().into(),
            ])
    }
    pub fn mosquito_variant_linear() -> Self {
        Self::mosquito_base()
            .with_x(SCREEN_RESOLUTION.x as f32 + 10.)
            .with_steps_vec(vec![
                EnemyStep::linear_movement_base().into(),
                EnemyStep::linear_movement_base()
                    .opposite_direction()
                    .into(),
            ])
    }
    pub fn mosquito_variant_linear_opposite() -> Self {
        Self::mosquito_base().with_x(-10.).with_steps_vec(vec![
            EnemyStep::linear_movement_base()
                .opposite_direction()
                .into(),
            EnemyStep::linear_movement_base().into(),
        ])
    }
    pub fn spidey_base(speed_multiplier: f32, coordinates: Vec2) -> Self {
        Self {
            enemy_type: EnemyType::Spidey,
            coordinates,
            depth: Depth::Six,
            elapsed: Duration::ZERO,
            steps: vec![EnemyStep::circle_around_base().opposite_direction().into()].into(),
            speed: speed_multiplier,
            contains: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, From, Reflect, Serialize)]
pub enum StageSpawn {
    Object(ObjectSpawn),
    Destructible(DestructibleSpawn),
    Pickup(PickupSpawn),
    Enemy(EnemySpawn),
}

impl StageSpawn {
    pub fn get_coordinates(&self) -> &Vec2 {
        match self {
            StageSpawn::Destructible(s) => &s.coordinates,
            StageSpawn::Enemy(s) => &s.coordinates,
            StageSpawn::Object(s) => &s.coordinates,
            StageSpawn::Pickup(s) => &s.coordinates,
        }
    }

    pub fn get_elapsed(&self) -> Duration {
        match self {
            StageSpawn::Destructible(_) => Duration::ZERO,
            StageSpawn::Enemy(s) => s.elapsed.div_f32(GAME_BASE_SPEED),
            StageSpawn::Object(_) => Duration::ZERO,
            StageSpawn::Pickup(s) => s.elapsed.div_f32(GAME_BASE_SPEED),
        }
    }

    pub fn get_depth(&self) -> Depth {
        match self {
            StageSpawn::Destructible(DestructibleSpawn { depth, .. }) => *depth,
            StageSpawn::Enemy(EnemySpawn { depth, .. }) => *depth,
            StageSpawn::Object(ObjectSpawn { depth, .. }) => *depth,
            StageSpawn::Pickup(PickupSpawn { depth, .. }) => *depth,
        }
    }

    pub fn show_type(&self) -> String {
        match self {
            StageSpawn::Destructible(s) => s.show_type(),
            StageSpawn::Enemy(s) => s.enemy_type.show_type(),
            StageSpawn::Object(s) => s.show_type(),
            StageSpawn::Pickup(s) => s.show_type(),
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum StageActionResumeCondition {
    MaxDuration(u32),
    KillAll,
    KillBoss,
}

#[derive(Clone, Debug, Deserialize, From, Reflect, Serialize)]
pub enum StageStep {
    Cinematic(CinematicStageStep),
    Movement(MovementStageStep),
    Stop(StopStageStep),
}

#[derive(Asset, Clone, Debug, Deserialize, Reflect, Resource, Serialize)]
pub struct StageData {
    pub name: String,
    pub background_path: String,
    pub music_path: String,
    pub skybox: SkyboxData,
    pub start_coordinates: Option<Vec2>,
    pub spawns: Vec<StageSpawn>,
    pub steps: Vec<StageStep>,
}
