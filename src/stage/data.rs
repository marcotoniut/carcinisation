use std::collections::VecDeque;

use bevy::{
    prelude::Vec2,
    reflect::{TypePath, TypeUuid},
};

use crate::{
    cinemachine::data::CinemachineData, globals::HALF_SCREEN_RESOLUTION,
    plugins::movement::structs::MovementDirection,
};

lazy_static! {
    pub static ref DEFAULT_COORDINATES: Vec2 = HALF_SCREEN_RESOLUTION.clone();
}

pub trait Contains {
    fn set_contains(&mut self, value: Option<Box<ContainerSpawn>>);
    fn drops(&mut self, value: ContainerSpawn);
}

#[derive(Debug, Clone)]
pub struct SkyboxData {
    pub path: String,
    pub frames: usize,
}

#[derive(Clone, Debug)]
pub enum DestructibleType {
    Lamp,
    Trashcan,
    Crystal,
    Mushroom,
    // Window,
    // Plant,
}

// deriving Default for simplicity's sake in defining the stage data
#[derive(Clone, Debug)]
pub enum ObjectType {
    BenchBig,
    BenchSmall,
    Fibertree,
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

// Should rename to EnemyBehavior?
#[derive(Clone, Copy, Debug)]
pub enum EnemyStep {
    Attack {
        duration: f32,
    },
    Circle {
        radius: f32,
        direction: MovementDirection,
        duration: f32,
    },
    Idle {
        duration: f32,
    },
    LinearMovement {
        coordinates: Vec2,
        attacking: bool,
        speed: f32,
    },
    Jump {
        coordinates: Vec2,
        attacking: bool,
        speed: f32,
    },
}

impl Default for EnemyStep {
    fn default() -> Self {
        EnemyStep::Idle {
            duration: Self::max_duration(),
        }
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
            EnemyStep::Attack { duration, .. } => Some(*duration),
            EnemyStep::Circle { duration, .. } => Some(*duration),
            EnemyStep::Idle { duration, .. } => Some(*duration),
            EnemyStep::LinearMovement { .. } => None,
            EnemyStep::Jump { .. } => None,
        }
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
}

impl PickupSpawn {
    pub fn set_elapsed(mut self, value: f32) -> Self {
        self.elapsed = value;
        self
    }
    pub fn set_coordinates(mut self, value: Vec2) -> Self {
        self.coordinates = value;
        self
    }
    pub fn big_healthpack_base() -> PickupSpawn {
        PickupSpawn {
            pickup_type: PickupType::BigHealthpack,
            coordinates: Vec2::ZERO,
            elapsed: 0.0,
        }
    }
    pub fn small_healthpack_base() -> PickupSpawn {
        PickupSpawn {
            pickup_type: PickupType::SmallHealthpack,
            coordinates: Vec2::ZERO,
            elapsed: 0.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DestructibleSpawn {
    pub destructible_type: DestructibleType,
    pub coordinates: Vec2,
    pub contains: Option<Box<ContainerSpawn>>,
}

impl DestructibleSpawn {
    pub fn set_coordinates(mut self, value: Vec2) -> Self {
        self.coordinates = value;
        self
    }
    pub fn set_contains(mut self, value: Option<Box<ContainerSpawn>>) -> Self {
        self.contains = value;
        self
    }
    pub fn drops(mut self, value: ContainerSpawn) -> Self {
        self.contains = Some(Box::new(value));
        self
    }

    pub fn lamp_base(x: f32, y: f32) -> DestructibleSpawn {
        DestructibleSpawn {
            destructible_type: DestructibleType::Lamp,
            coordinates: Vec2::new(x, y),
            contains: None,
        }
    }

    pub fn trashcan_base(x: f32, y: f32) -> DestructibleSpawn {
        DestructibleSpawn {
            destructible_type: DestructibleType::Trashcan,
            coordinates: Vec2::new(x, y),
            contains: None,
        }
    }

    pub fn crystal_base(x: f32, y: f32) -> DestructibleSpawn {
        DestructibleSpawn {
            destructible_type: DestructibleType::Crystal,
            coordinates: Vec2::new(x, y),
            contains: None,
        }
    }

    pub fn mushroom_base(x: f32, y: f32) -> DestructibleSpawn {
        DestructibleSpawn {
            destructible_type: DestructibleType::Mushroom,
            coordinates: Vec2::new(x, y),
            contains: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ObjectSpawn {
    pub object_type: ObjectType,
    pub coordinates: Vec2,
}

impl ObjectSpawn {
    pub fn set_coordinates(mut self, value: Vec2) -> Self {
        self.coordinates = value;
        self
    }

    pub fn bench_big_base(x: f32, y: f32) -> ObjectSpawn {
        ObjectSpawn {
            object_type: ObjectType::BenchBig,
            coordinates: Vec2::new(x, y),
        }
    }

    pub fn bench_small_base(x: f32, y: f32) -> ObjectSpawn {
        ObjectSpawn {
            object_type: ObjectType::BenchSmall,
            coordinates: Vec2::new(x, y),
        }
    }

    pub fn fibertree_base(x: f32, y: f32) -> ObjectSpawn {
        ObjectSpawn {
            object_type: ObjectType::Fibertree,
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
}

impl EnemySpawn {
    pub fn set_elapsed(mut self, value: f32) -> Self {
        self.elapsed = value;
        self
    }
    pub fn set_coordinates(mut self, value: Vec2) -> Self {
        self.coordinates = value;
        self
    }
    pub fn set_speed(mut self, value: f32) -> Self {
        self.speed = value;
        self
    }
    pub fn set_steps(mut self, value: VecDeque<EnemyStep>) -> Self {
        self.steps = value;
        self
    }
    pub fn set_steps_vec(mut self, value: Vec<EnemyStep>) -> Self {
        self.steps = value.into();
        self
    }
    /** TODO should I implement these as a trait Contains */
    pub fn set_contains(mut self, value: Option<Box<ContainerSpawn>>) -> Self {
        self.contains = value;
        self
    }
    pub fn drops(mut self, value: ContainerSpawn) -> Self {
        self.contains = Some(Box::new(value));
        self
    }
    pub fn tardigrade_base() -> EnemySpawn {
        EnemySpawn {
            enemy_type: EnemyType::Tardigrade,
            coordinates: DEFAULT_COORDINATES.clone(),
            speed: 5.0,
            elapsed: 0.0,
            steps: vec![].into(),
            contains: None,
        }
    }
    pub fn mosquito_base() -> EnemySpawn {
        EnemySpawn {
            enemy_type: EnemyType::Mosquito,
            coordinates: DEFAULT_COORDINATES.clone(),
            speed: 40.0,
            elapsed: 0.0,
            steps: vec![].into(),
            contains: None,
        }
    }
    pub fn spidey_base(speed_multiplier: f32, coordinates: Vec2) -> EnemySpawn {
        EnemySpawn {
            enemy_type: EnemyType::Spidey,
            speed: speed_multiplier,
            coordinates,
            elapsed: 0.0,
            steps: vec![EnemyStep::Circle {
                duration: 999.,
                radius: 12.,
                direction: MovementDirection::Positive,
            }]
            .into(),
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
    pub fn get_elapsed(&self) -> f32 {
        match self {
            StageSpawn::Destructible(DestructibleSpawn { .. }) => 0.,
            StageSpawn::Enemy(EnemySpawn { elapsed, .. }) => *elapsed,
            StageSpawn::Object(ObjectSpawn { .. }) => 0.,
            StageSpawn::Pickup(PickupSpawn { elapsed, .. }) => *elapsed,
        }
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

#[derive(Clone, Debug)]
pub enum StageActionResumeCondition {
    KillAll,
    KillBoss,
}

#[derive(Clone, Debug)]
pub enum StageStep {
    Cinematic {
        cinematic: CinemachineData,
    },
    Movement {
        coordinates: Vec2,
        base_speed: f32,
        spawns: Vec<StageSpawn>,
    },
    Stop {
        resume_conditions: Option<Vec<StageActionResumeCondition>>,
        max_duration: Option<f32>,
        spawns: Vec<StageSpawn>,
    },
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
