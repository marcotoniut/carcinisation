use bevy::prelude::*;

use crate::stage::data::{
    ContainerSpawn, DestructibleSpawn, EnemySpawn, ObjectSpawn, ObjectType,
    SkyboxData, StageData, StageStep,
};
use crate::stage::data::{
    DestructibleType, EnemyStep, EnemyType, StageActionResumeCondition, StageSpawn,
};

use crate::cinemachine::*;

use bevy::prelude::*;
use lazy_static::lazy_static;

use super::data::Clip;

lazy_static! {
    pub static ref INTRO_ANIMATIC: CinemachineData = CinemachineData {
        name: "intro".to_string(),
        clip: Clip{
            frame_count: 0,
            frame_duration_millis: 2000,
            image_path: "/cinematics/intro/0.png".to_string(),
            start_coordinates: Vec2{x:0.0,y:0.0},
            layer_index: 100.0, 
            snd: None,
            waitInSeconds: 3.0,
        }
    };
}