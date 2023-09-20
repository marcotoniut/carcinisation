mod paths;

extern crate serde;
extern crate serde_yaml;

use paths::ASSETS_STAGES_PATH;
use serde::Deserialize;
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct Vec2(f32, f32);

#[derive(Debug, Deserialize)]
pub enum DestructibleType {
    Lamp,
    Window,
}

#[derive(Debug, Deserialize)]
pub enum PowerupType {
    Health,
}

#[derive(Debug, Deserialize)]
pub enum EnemyType {
    Mosquito,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum Spawn {
    Destructible {
        destructible_type: DestructibleType,
        coordinates: Vec2,
    },
    Powerup {
        powerup_type: PowerupType,
        coordinates: Vec2,
    },
    Enemy {
        enemy_type: EnemyType,
        coordinates: Vec2,
        base_speed: f32,
        time: f32,
        special_path: Option<String>,
    },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    Movement {
        coordinates: Vec2,
        base_speed: Option<f32>,
        spawns: Option<Vec<Spawn>>,
    },
    Stop {
        resume_conditions: Option<Vec<String>>,
        max_duration: Option<f32>,
        spawns: Option<Vec<Spawn>>,
    },
}

#[derive(Debug, Deserialize)]
pub struct Stage {
    pub name: String,
    pub background: String,
    pub skybox: String,
    pub start_coordinates: Vec2,
    pub spawns: Vec<Spawn>,
    pub actions: Vec<Action>,
}

fn main() {
    let path = Path::new(ASSETS_STAGES_PATH);

    for entry in path.read_dir().expect("read_dir call failed") {
        if let Ok(entry) = entry {
            if let Ok(file) = File::open(entry.path()) {
                let result: Result<Stage, serde_yaml::Error> = serde_yaml::from_reader(file);
                match result {
                    Ok(stage) => println!("{:#?}", stage),
                    Err(e) => eprintln!("Error parsing file {:?}: {}", entry.path(), e),
                }
            }
        }
    }
}
