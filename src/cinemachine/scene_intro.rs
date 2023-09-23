use bevy::prelude::*;

use crate::stage::data::{
    ContainerSpawn, DestructibleSpawn, EnemySpawn, ObjectSpawn, ObjectType, PowerupSpawn,
    SkyboxData, StageData, StageStep,
};
use crate::stage::data::{
    DestructibleType, EnemyStep, EnemyType, PowerupType, StageActionResumeCondition, StageSpawn,
};

use crate::cinemachine::*;

use bevy::prelude::*;
use lazy_static::lazy_static;

use super::data::Clip;

lazy_static! {
    pub static ref INTRO_ANIMATIC: CinemachineData = CinemachineData {
        name: "intro".to_string(),
        start_coordinates: Vec2::new(0.0, 0.0),
        clips: make_clips()
    };
}

fn make_clips() -> Vec<Clip>{
    vec![
        Clip{
            image_path: Some("/cinematics/intro/0.png".to_string()),
            foreground_elements: None,
            start_coordinates: Vec2{x:0.0,y:0.0},
            simple_pathing: None,
            layer_index: 0.0, 
            snd: None, 
            wait: 1.0
        }
    ]
}