use bevy::{
    core_pipeline::Skybox,
    prelude::{Handle, Resource, Vec2},
    reflect::{TypePath, TypeUuid},
};
use serde::Deserialize;

use crate::resource::asset_data::SkyboxData;

#[derive(Clone, Debug, Default, Deserialize)]
pub enum DestructibleType {
    #[default]
    Lamp,
    Window,
    Plant,
}

// deriving Default for simplicity's sake in defining the stage data
#[derive(Clone, Debug, Default, Deserialize)]
pub enum PowerupType {
    #[default]
    SmallHealthpack,
    BigHealthpack,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub enum EnemyType {
    #[default]
    Mosquito,
    Spidey,
    Tardigrade,
    Marauder,
    Spidomonsta,
    Kyle,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type")]
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

#[derive(Clone, Debug, Deserialize)]
pub enum ContainerSpawn {
    Powerup(PowerupSpawn),
    Enemy(EnemySpawn),
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct PowerupSpawn {
    pub powerup_type: PowerupType,
    #[serde(default = "default_vec2_zero")]
    pub coordinates: Vec2,
    #[serde(default = "default_zero")]
    pub elapsed: f32,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct DestructibleSpawn {
    pub destructible_type: DestructibleType,
    pub coordinates: Vec2,
    #[serde(default = "default_zero")]
    pub elapsed: f32,
    #[serde(default = "default_none")]
    pub contains: Option<Box<ContainerSpawn>>,
}

#[derive(Clone, Debug, Default, Deserialize)]
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

#[derive(Clone, Debug, Deserialize)]
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

#[derive(Clone, Debug, Default, Deserialize)]
pub enum StageActionResumeCondition {
    #[default]
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
    pub skybox: Option<SkyboxData>,
    pub start_coordinates: Option<Vec2>,
    #[serde(default = "empty_vec")]
    pub spawns: Vec<StageSpawn>,
    pub steps: Vec<StageStep>,
}
