use std::{collections::VecDeque, time::Duration};

use bevy::{
    prelude::Vec2,
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

#[derive(Clone, Copy, Debug)]
pub struct AttackEnemyStep {
    pub duration: f32,
}

impl AttackEnemyStep {
    pub fn base() -> Self {
        Self {
            duration: EnemyStep::max_duration(),
        }
    }

    pub fn with_duration(mut self, value: f32) -> Self {
        self.duration = value;
        self
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CircleEnemyStep {
    pub depth_movement_o: Option<i8>,
    pub direction: MovementDirection,
    pub duration: f32,
    pub radius: f32,
}

impl CircleEnemyStep {
    pub fn base() -> Self {
        Self {
            depth_movement_o: None,
            direction: MovementDirection::Negative,
            duration: EnemyStep::max_duration(),
            radius: 12.,
        }
    }

    pub fn opposite_direction(mut self) -> Self {
        self.direction = self.direction.opposite();
        self
    }

    pub fn with_depth_movement(mut self, value: i8) -> Self {
        self.depth_movement_o = Some(value);
        self
    }

    pub fn without_depth_movement(mut self) -> Self {
        self.depth_movement_o = None;
        self
    }

    pub fn with_direction(mut self, value: MovementDirection) -> Self {
        self.direction = value;
        self
    }

    pub fn with_duration(mut self, value: f32) -> Self {
        self.duration = value;
        self
    }

    pub fn with_radius(mut self, value: f32) -> Self {
        self.radius = value;
        self
    }
}

#[derive(Clone, Copy, Debug)]
pub struct IdleEnemyStep {
    pub duration: f32,
}

impl IdleEnemyStep {
    pub fn base() -> Self {
        Self {
            duration: EnemyStep::max_duration(),
        }
    }

    pub fn with_duration(mut self, value: f32) -> Self {
        self.duration = value;
        self
    }
}

impl Default for IdleEnemyStep {
    fn default() -> Self {
        Self::base()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LinearMovementEnemyStep {
    pub depth_movement_o: Option<i8>,
    pub direction: Vec2,
    pub trayectory: f32,
}

impl LinearMovementEnemyStep {
    pub fn base() -> Self {
        Self {
            direction: Vec2::new(-1., 0.),
            depth_movement_o: None,
            trayectory: 0.,
        }
    }

    pub fn opposite_direction(mut self) -> Self {
        self.direction = Vec2::new(-self.direction.x, -self.direction.y);
        self
    }

    pub fn with_direction(mut self, value: Vec2) -> Self {
        self.direction = value;
        self
    }

    pub fn with_trayectory(mut self, value: f32) -> Self {
        self.trayectory = value;
        self
    }

    pub fn with_depth_movement(mut self, value: i8) -> Self {
        self.depth_movement_o = Some(value);
        self
    }
}

#[derive(Clone, Copy, Debug)]
pub struct JumpEnemyStep {
    pub attacking: bool,
    pub coordinates: Vec2,
    pub depth_movement: Option<i8>,
    pub speed: f32,
}

impl JumpEnemyStep {
    pub fn base() -> Self {
        Self {
            coordinates: Vec2::ZERO,
            attacking: false,
            depth_movement: None,
            speed: 0.5,
        }
    }
}

// Should rename to EnemyBehavior?
#[derive(Clone, Copy, Debug)]
pub enum EnemyStep {
    Attack(AttackEnemyStep),
    Circle(CircleEnemyStep),
    Idle(IdleEnemyStep),
    LinearMovement(LinearMovementEnemyStep),
    Jump(JumpEnemyStep),
}

impl Default for EnemyStep {
    fn default() -> Self {
        IdleEnemyStep::default().into()
    }
}

impl EnemyStep {
    pub fn max_duration() -> f32 {
        99999.
    }

    pub fn get_duration(&self) -> f32 {
        self.get_duration_o()
            .unwrap_or_else(|| EnemyStep::max_duration())
    }

    pub fn get_duration_o(&self) -> Option<f32> {
        match self {
            EnemyStep::Attack(AttackEnemyStep { duration, .. }) => Some(*duration),
            EnemyStep::Circle(CircleEnemyStep { duration, .. }) => Some(*duration),
            EnemyStep::Idle(IdleEnemyStep { duration, .. }) => Some(*duration),
            EnemyStep::LinearMovement { .. } => None,
            EnemyStep::Jump { .. } => None,
        }
    }

    pub fn attack_base() -> AttackEnemyStep {
        AttackEnemyStep::base()
    }

    pub fn circle_around_base() -> CircleEnemyStep {
        CircleEnemyStep::base()
    }

    pub fn idle_base() -> IdleEnemyStep {
        IdleEnemyStep::base()
    }

    pub fn jump_base() -> JumpEnemyStep {
        JumpEnemyStep::base()
    }

    pub fn linear_movement_base() -> LinearMovementEnemyStep {
        LinearMovementEnemyStep::base()
    }
}

impl From<AttackEnemyStep> for EnemyStep {
    fn from(step: AttackEnemyStep) -> Self {
        EnemyStep::Attack(step)
    }
}

impl From<IdleEnemyStep> for EnemyStep {
    fn from(step: IdleEnemyStep) -> Self {
        EnemyStep::Idle(step)
    }
}

impl From<CircleEnemyStep> for EnemyStep {
    fn from(step: CircleEnemyStep) -> Self {
        EnemyStep::Circle(step)
    }
}

impl From<LinearMovementEnemyStep> for EnemyStep {
    fn from(step: LinearMovementEnemyStep) -> Self {
        EnemyStep::LinearMovement(step)
    }
}

impl From<JumpEnemyStep> for EnemyStep {
    fn from(step: JumpEnemyStep) -> Self {
        EnemyStep::Jump(step)
    }
}

#[derive(Clone, Debug)]
pub enum ContainerSpawn {
    Pickup(PickupSpawn),
    Enemy(EnemySpawn),
}

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

    pub fn movement_base(x: f32, y: f32) -> Self {
        StageStep::Movement(MovementStageStep {
            coordinates: Vec2::new(x, y),
            base_speed: 1.,
            spawns: vec![],
            floor_depths: None,
        })
    }
}

#[derive(TypeUuid, TypePath, Clone, Debug)]
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
