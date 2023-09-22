use std::ptr::null;

use bevy::prelude::*;
use crate::stage::data::{StageStep, StageSpawn, EnemyType, EnemyStep};

use super::asset_data::*;

pub const ASTEROID_DATA: AssetData<'static> = AssetData {
    name: "Asteroid",
    background: "backgrounds/stage_asteroid/cavern.png",
    skybox: Some("backgrounds/stage_asteroid/space.png"),
    start_coordinates: Some(Vec2::new(0.0,0.0)),
    spawns: vec![],//Vec::new(),
    steps: vec![],
    _set_steps: _set_steps,
};

pub fn _set_steps(){
    ASTEROID_DATA.steps.clear();

    ASTEROID_DATA.steps
    .push(
        StageStep::Movement { 
            coordinates: Vec2 { x: 50.0, y: 0.0 }, 
            base_speed: 10.0, 
            spawns: vec![
                StageSpawn::Enemy { 
                    enemy_type: EnemyType::Mosquito, 
                    coordinates: Vec2 { x: 60.0, y: 100.0 }, 
                    base_speed: 5.0, 
                    elapsed: 1.4, 
                    steps: vec![
                        EnemyStep::Movement { 
                            coordinates: Vec2 {x: 50.0, y: 0.0 },
                            attacking: true, 
                            speed: 5.0 
                        }, 
                        EnemyStep::Stop { 
                            duration: 1.0
                        }, 
                        EnemyStep::Attack { 
                            duration: 1.0 
                        }, 
                        EnemyStep::Movement {
                            coordinates: Vec2 {x: 10.0, y: 0.0 },
                            attacking: true, 
                            speed: 3.0 
                        }, 
                        EnemyStep::CircleAround { 
                            duration: 4.0
                        }
                    ] 
                }
            ]
        }
    );
}