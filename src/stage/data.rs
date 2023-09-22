use bevy::{
    prelude::{Handle, Resource, Vec2},
    reflect::{TypePath, TypeUuid},
};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub enum DestructibleType {
    Lamp,
    Window,
    Plant,
}

#[derive(Debug, Deserialize, Clone)]
pub enum PowerupType {
    SmallHealthpack,
    BigHealthpack,
}

#[derive(Debug, Deserialize, Clone)]
pub enum EnemyType {
    Mosquito,
    Spidey,
    Tardigrade,
    Marauder,
    Spidomonsta,
    Kyle,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum EnemyStep {
    Movement {
        coordinates: Vec2,
        attacking: bool,
        speed: f32,
    },
    Stop {
        duration: f32,
    },
    Attack {
        duration: f32,
    },
    CircleAround {
        duration: f32,
    },
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

#[derive(Debug, Deserialize, Clone)]
pub enum ContainerSpawn {
    Powerup(PowerupSpawn),
    Enemy(EnemySpawn),
}

#[derive(Debug, Deserialize, Clone)]
pub struct PowerupSpawn {
    pub powerup_type: PowerupType,
    pub coordinates: Vec2,
    #[serde(default = "default_zero")]
    pub elapsed: f32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DestructibleSpawn {
    pub destructible_type: DestructibleType,
    pub coordinates: Vec2,
    #[serde(default = "default_zero")]
    pub elapsed: f32,
    #[serde(default = "default_none")]
    pub contains: Option<Box<ContainerSpawn>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct EnemySpawn {
    pub enemy_type: EnemyType,
    pub coordinates: Vec2,
    pub base_speed: f32,
    #[serde(default = "default_zero")]
    pub elapsed: f32,
    #[serde(default = "empty_vec")]
    pub steps: Vec<EnemyStep>,
    pub contains: Option<Box<ContainerSpawn>>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type")]
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

#[derive(Debug, Deserialize, Clone)]
pub enum StageActionResumeCondition {
    KillAll,
    KillBoss,
}

fn default_base_speed() -> f32 {
    1.0
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type")]
pub enum StageStep {
    Movement {
        coordinates: Vec2,
        #[serde(default = "default_base_speed")]
        base_speed: f32,
        #[serde(default = "empty_vec")]
        spawns: Vec<StageSpawn>,
    },
    Stop {
        resume_conditions: Option<Vec<StageActionResumeCondition>>,
        max_duration: Option<u64>,
        #[serde(default = "empty_vec")]
        spawns: Vec<StageSpawn>,
    },
}

#[derive(Deserialize, TypeUuid, TypePath, Clone, Debug)]
#[uuid = "c17075ed-7df0-4a51-b961-ce5270a8a934"]
pub struct StageData {
    pub name: String,
    pub background: String,
    pub skybox: Option<String>,
    pub start_coordinates: Option<Vec2>,
    #[serde(default = "empty_vec")]
    pub spawns: Vec<StageSpawn>,
    pub steps: Vec<StageStep>,
}
