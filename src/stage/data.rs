use std::{collections::VecDeque, time::Duration};

use bevy::{
    prelude::{Resource, Vec2},
    reflect::{TypePath, TypeUuid},
    utils::HashMap,
};

use crate::{
    cinemachine::data::CinemachineData,
    globals::{HALF_SCREEN_RESOLUTION, SCREEN_RESOLUTION},
    plugins::movement::structs::MovementDirection,
};

use super::{
    components::{CinematicStageStep, MovementStageStep, StopStageStep},
    destructible::data::DestructibleSpawn,
    enemy::data::steps::EnemyStep,
};

lazy_static! {
    pub static ref DEFAULT_COORDINATES: Vec2 = HALF_SCREEN_RESOLUTION.clone();
}

pub const GAME_BASE_SPEED: f32 = 15.0;

pub trait Contains {
    fn with_contains(&mut self, value: Option<Box<ContainerSpawn>>);
    fn drops(&mut self, value: ContainerSpawn);
}

#[derive(Debug, Clone)]
pub struct SkyboxData {
    pub path: String,
    pub frames: usize,
}

// deriving Default for simplicity's sake in defining the stage data
#[derive(Clone, Debug)]
pub enum ObjectType {
    BenchBig,
    BenchSmall,
    Fibertree,
    Rugpark,
}

#[derive(Clone, Debug)]
pub enum PickupType {
    SmallHealthpack,
    BigHealthpack,
    // TODO
    // Weapon,
    // Ammo,
    // Shield,
}

#[derive(Clone, Copy, Debug, Default)]
pub enum EnemyType {
    #[default]
    Mosquito,
    Spidey,
    Tardigrade,
    Marauder,
    Spidomonsta,
    Kyle,
}

#[derive(Clone, Debug)]
pub enum ContainerSpawn {
    Pickup(PickupSpawn),
    Enemy(EnemySpawn),
}

impl From<PickupSpawn> for ContainerSpawn {
    fn from(value: PickupSpawn) -> Self {
        ContainerSpawn::Pickup(value)
    }
}

impl From<EnemySpawn> for ContainerSpawn {
    fn from(value: EnemySpawn) -> Self {
        ContainerSpawn::Enemy(value)
    }
}

// TODO move pickup data under its own module?
#[derive(Clone, Debug)]
pub struct PickupSpawn {
    pub pickup_type: PickupType,
    pub coordinates: Vec2,
    pub elapsed: f32,
    pub depth: u8,
}

impl PickupSpawn {
    pub fn with_elapsed(mut self, value: f32) -> Self {
        self.elapsed = value;
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
            elapsed: 0.0,
            depth: 3,
        }
    }
    pub fn small_healthpack_base() -> Self {
        Self {
            pickup_type: PickupType::SmallHealthpack,
            coordinates: Vec2::ZERO,
            elapsed: 0.0,
            depth: 3,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ObjectSpawn {
    pub object_type: ObjectType,
    pub coordinates: Vec2,
}

impl ObjectSpawn {
    pub fn with_coordinates(mut self, value: Vec2) -> Self {
        self.coordinates = value;
        self
    }

    pub fn bench_big_base(x: f32, y: f32) -> Self {
        Self {
            object_type: ObjectType::BenchBig,
            coordinates: Vec2::new(x, y),
        }
    }

    pub fn bench_small_base(x: f32, y: f32) -> Self {
        Self {
            object_type: ObjectType::BenchSmall,
            coordinates: Vec2::new(x, y),
        }
    }

    pub fn fibertree_base(x: f32, y: f32) -> Self {
        Self {
            object_type: ObjectType::Fibertree,
            coordinates: Vec2::new(x, y),
        }
    }

    pub fn rugpark_sign_base(x: f32, y: f32) -> Self {
        Self {
            object_type: ObjectType::Rugpark,
            coordinates: Vec2::new(x, y),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct EnemySpawn {
    pub enemy_type: EnemyType,
    pub elapsed: f32,
    pub contains: Option<Box<ContainerSpawn>>,
    pub coordinates: Vec2,
    pub speed: f32,
    pub steps: VecDeque<EnemyStep>,
    pub depth: u8,
}

impl EnemySpawn {
    pub fn with_elapsed(mut self, value: f32) -> Self {
        self.elapsed = value;
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
    pub fn with_depth(mut self, value: u8) -> Self {
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
            depth: 3,
            elapsed: 0.0,
            speed: 0.5,
            steps: vec![].into(),
            contains: None,
        }
    }
    pub fn mosquito_base() -> Self {
        Self {
            enemy_type: EnemyType::Mosquito,
            coordinates: *DEFAULT_COORDINATES,
            depth: 4,
            elapsed: 0.0,
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
            depth: 3,
            elapsed: 0.0,
            steps: vec![EnemyStep::circle_around_base().opposite_direction().into()].into(),
            speed: speed_multiplier,
            contains: None,
        }
    }
}

#[derive(Clone, Debug)]
pub enum StageSpawn {
    Object(ObjectSpawn),
    Destructible(DestructibleSpawn),
    Pickup(PickupSpawn),
    Enemy(EnemySpawn),
}

impl StageSpawn {
    pub fn get_elapsed(&self) -> Duration {
        Duration::from_secs_f32(match self {
            StageSpawn::Destructible(DestructibleSpawn { .. }) => 0.,
            StageSpawn::Enemy(EnemySpawn { elapsed, .. }) => *elapsed / GAME_BASE_SPEED,
            StageSpawn::Object(ObjectSpawn { .. }) => 0.,
            StageSpawn::Pickup(PickupSpawn { elapsed, .. }) => *elapsed / GAME_BASE_SPEED,
        })
    }

    pub fn show_spawn_type(&self) -> String {
        match self {
            StageSpawn::Destructible(DestructibleSpawn {
                destructible_type, ..
            }) => {
                format!("Destructible({:?})", destructible_type)
            }
            StageSpawn::Enemy(EnemySpawn { enemy_type, .. }) => format!("Enemy({:?})", enemy_type),
            StageSpawn::Object(ObjectSpawn { object_type, .. }) => {
                format!("Object({:?})", object_type)
            }
            StageSpawn::Pickup(PickupSpawn { pickup_type, .. }) => {
                format!("Pickup({:?})", pickup_type)
            }
        }
    }
}

impl From<ObjectSpawn> for StageSpawn {
    fn from(value: ObjectSpawn) -> Self {
        StageSpawn::Object(value)
    }
}

impl From<DestructibleSpawn> for StageSpawn {
    fn from(value: DestructibleSpawn) -> Self {
        StageSpawn::Destructible(value)
    }
}

impl From<PickupSpawn> for StageSpawn {
    fn from(value: PickupSpawn) -> Self {
        StageSpawn::Pickup(value)
    }
}

impl From<EnemySpawn> for StageSpawn {
    fn from(value: EnemySpawn) -> Self {
        StageSpawn::Enemy(value)
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum StageActionResumeCondition {
    MaxDuration(u32),
    KillAll,
    KillBoss,
}

#[derive(Clone, Debug)]
pub enum StageStep {
    Cinematic(CinematicStageStep),
    Movement(MovementStageStep),
    Stop(StopStageStep),
}

impl From<CinematicStageStep> for StageStep {
    fn from(value: CinematicStageStep) -> Self {
        Self::Cinematic(value)
    }
}

impl From<MovementStageStep> for StageStep {
    fn from(value: MovementStageStep) -> Self {
        Self::Movement(value)
    }
}

impl From<StopStageStep> for StageStep {
    fn from(value: StopStageStep) -> Self {
        Self::Stop(value)
    }
}

impl StageStep {
    pub fn speed(&self) -> f32 {
        match self {
            StageStep::Movement(MovementStageStep { base_speed, .. }) => {
                *base_speed * GAME_BASE_SPEED
            }
            StageStep::Stop(StopStageStep { .. }) => 0.,
            StageStep::Cinematic(CinematicStageStep { .. }) => 0.,
        }
    }

    pub fn add_spawns(mut self, new_spawns: Vec<StageSpawn>) -> Self {
        match &mut self {
            StageStep::Movement(MovementStageStep { spawns, .. }) => {
                spawns.extend(new_spawns);
            }
            StageStep::Stop(StopStageStep { spawns, .. }) => {
                spawns.extend(new_spawns);
            }
            StageStep::Cinematic(CinematicStageStep { .. }) => {}
        };
        self
    }

    pub fn with_base_speed(mut self, base_speed: f32) -> Self {
        if let StageStep::Movement(MovementStageStep {
            base_speed: ref mut s,
            ..
        }) = self
        {
            *s = base_speed;
        }
        self
    }

    pub fn with_floor_depths(mut self, floor_depths: HashMap<u8, f32>) -> Self {
        match &mut self {
            StageStep::Movement(MovementStageStep {
                floor_depths: ref mut f,
                ..
            }) => {
                *f = Some(floor_depths);
            }
            StageStep::Stop(StopStageStep {
                floor_depths: ref mut f,
                ..
            }) => {
                *f = Some(floor_depths);
            }
            _ => {}
        };
        self
    }

    pub fn movement_base(x: f32, y: f32) -> MovementStageStep {
        MovementStageStep::base(x, y)
    }
}

#[derive(TypeUuid, TypePath, Clone, Debug, Resource)]
#[uuid = "c17075ed-7df0-4a51-b961-ce5270a8a934"]
pub struct StageData {
    pub name: String,
    pub background_path: String,
    pub music_path: String,
    pub skybox: SkyboxData,
    pub start_coordinates: Option<Vec2>,
    pub spawns: Vec<StageSpawn>,
    pub steps: Vec<StageStep>,
}
