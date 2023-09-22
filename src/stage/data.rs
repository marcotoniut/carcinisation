use bevy::{
    prelude::Vec2,
    reflect::{TypePath, TypeUuid},
};

#[derive(Debug, Clone)]
pub struct SkyboxData {
    pub path: String,
    pub frames: usize,
}

#[derive(Clone, Debug, Default)]
pub enum DestructibleType {
    #[default]
    Lamp,
    Window,
    Plant,
}

// deriving Default for simplicity's sake in defining the stage data
#[derive(Clone, Debug, Default)]
pub enum PowerupType {
    #[default]
    SmallHealthpack,
    BigHealthpack,
}

#[derive(Clone, Debug, Default)]
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
pub enum EnemyStep {
    Attack {
        duration: f32,
    },
    Circle {
        duration: f32,
    },
    Idle {
        duration: f32,
    },
    Movement {
        coordinates: Vec2,
        attacking: bool,
        speed: f32,
    },
}

impl Default for EnemyStep {
    fn default() -> Self {
        EnemyStep::Idle { duration: 0.0 }
    }
}

fn default_vec2_zero() -> Vec2 {
    Vec2::new(0.0, 0.0)
}

fn default_zero() -> f32 {
    0.0
}

fn default_none<T>() -> Option<T> {
    None
}

fn empty_vec<T: Clone>() -> Vec<T> {
    [].to_vec()
}

#[derive(Clone, Debug)]
pub enum ContainerSpawn {
    Powerup(PowerupSpawn),
    Enemy(EnemySpawn),
}

#[derive(Clone, Debug, Default)]
pub struct PowerupSpawn {
    pub powerup_type: PowerupType,
    pub coordinates: Vec2,
    pub elapsed: f32,
}

#[derive(Clone, Debug, Default)]
pub struct DestructibleSpawn {
    pub destructible_type: DestructibleType,
    pub coordinates: Vec2,
    pub elapsed: f32,
    pub contains: Option<Box<ContainerSpawn>>,
}

#[derive(Clone, Debug, Default)]
pub struct EnemySpawn {
    pub enemy_type: EnemyType,
    pub coordinates: Vec2,
    pub base_speed: f32,
    pub elapsed: f32,
    pub steps: Vec<EnemyStep>,
    pub contains: Option<Box<ContainerSpawn>>,
}

#[derive(Clone, Debug)]
pub enum StageSpawn {
    Destructible(DestructibleSpawn),
    Powerup(PowerupSpawn),
    Enemy(EnemySpawn),
}

impl StageSpawn {
    pub fn get_elapsed(&self) -> f32 {
        match self {
            StageSpawn::Destructible(DestructibleSpawn { elapsed, .. }) => *elapsed,
            StageSpawn::Powerup(PowerupSpawn { elapsed, .. }) => *elapsed,
            StageSpawn::Enemy(EnemySpawn { elapsed, .. }) => *elapsed,
        }
    }

    pub fn show_spawn_type(&self) -> String {
        match self {
            StageSpawn::Destructible(DestructibleSpawn {
                destructible_type, ..
            }) => {
                format!("Destructible({:?})", destructible_type)
            }
            StageSpawn::Powerup(PowerupSpawn { powerup_type, .. }) => {
                format!("Powerup({:?})", powerup_type)
            }
            StageSpawn::Enemy(EnemySpawn { enemy_type, .. }) => format!("Enemy({:?})", enemy_type),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub enum StageActionResumeCondition {
    #[default]
    KillAll,
    KillBoss,
}

#[derive(Clone, Debug)]
pub enum StageStep {
    Movement {
        coordinates: Vec2,
        base_speed: f32,
        spawns: Vec<StageSpawn>,
    },
    Stop {
        resume_conditions: Option<Vec<StageActionResumeCondition>>,
        max_duration: Option<u64>,
        spawns: Vec<StageSpawn>,
    },
}

#[derive(TypeUuid, TypePath, Clone, Debug)]
#[uuid = "c17075ed-7df0-4a51-b961-ce5270a8a934"]
pub struct StageData {
    pub name: String,
    pub background: String,
    pub skybox: SkyboxData,
    pub start_coordinates: Option<Vec2>,
    pub spawns: Vec<StageSpawn>,
    pub steps: Vec<StageStep>,
}
