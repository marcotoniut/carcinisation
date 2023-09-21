use bevy::{
    prelude::*,
    reflect::{TypePath, TypeUuid},
};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub enum DestructibleType {
    Lamp,
    Window,
}

#[derive(Debug, Deserialize, Clone)]
pub enum PowerupType {
    Health,
}

#[derive(Debug, Deserialize, Clone)]
pub enum EnemyType {
    Mosquito,
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

fn empty_vec<T: Clone>() -> Vec<T> {
    [].to_vec()
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum StageSpawn {
    Destructible {
        destructible_type: DestructibleType,
        coordinates: Vec2,
    },
    Powerup {
        powerup_type: PowerupType,
        coordinates: Vec2,
        elapsed: Option<f32>,
    },
    Enemy {
        enemy_type: EnemyType,
        coordinates: Vec2,
        base_speed: f32,
        elapsed: Option<f32>,
        #[serde(default = "empty_vec")]
        steps: Vec<EnemyStep>,
    },
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
        spawns: Option<Vec<StageSpawn>>,
    },
    Stop {
        resume_conditions: Option<Vec<StageActionResumeCondition>>,
        max_duration: Option<u64>,
        spawns: Option<Vec<StageSpawn>>,
    },
}

#[derive(Deserialize, TypeUuid, TypePath, Clone, Debug)]
#[uuid = "c17075ed-7df0-4a51-b961-ce5270a8a934"]
pub struct StageData {
    pub name: String,
    pub background: String,
    pub skybox: Option<String>,
    pub start_coordinates: Option<Vec2>,
    pub spawns: Vec<StageSpawn>,
    pub steps: Vec<StageStep>,
}
