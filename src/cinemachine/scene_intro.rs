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
    pub static ref INTRO_ANIMATIC_0: CinemachineData = CinemachineData {
        name: "intro".to_string(),
        clip: Clip{
            frame_count: 2,
            frame_duration_millis: 200,
            image_path: "/cinematics/intro/bald_guy.png".to_string(),
            start_coordinates: Vec2{x:0.0,y:0.0},
            layer_index: 100.0, 
            snd: None,
            waitInSeconds: 3.0,
        }
    };
    pub static ref INTRO_ANIMATIC_1: CinemachineData = CinemachineData {
        name: "intro".to_string(),
        clip: Clip{
            frame_count: 1,
            frame_duration_millis: 200,
            image_path: "/cinematics/intro/1.png".to_string(),
            start_coordinates: Vec2{x:0.0,y:0.0},
            layer_index: 100.0, 
            snd: None,
            waitInSeconds: 4.0,
        }
    };
    pub static ref INTRO_ANIMATIC_2: CinemachineData = CinemachineData {
        name: "intro".to_string(),
        clip: Clip{
            frame_count: 1,
            frame_duration_millis: 200,
            image_path: "/cinematics/intro/screaming_scene.png".to_string(),
            start_coordinates: Vec2{x:0.0,y:0.0},
            layer_index: 100.0, 
            snd: None,
            waitInSeconds: 2.0,
        }
    };
    pub static ref INTRO_ANIMATIC_3: CinemachineData = CinemachineData {
        name: "intro".to_string(),
        clip: Clip{
            frame_count: 1,
            frame_duration_millis: 200,
            image_path: "/cinematics/intro/transform.png".to_string(),
            start_coordinates: Vec2{x:0.0,y:0.0},
            layer_index: 100.0, 
            snd: None,
            waitInSeconds: 2.0,
        }
    };
    pub static ref INTRO_ANIMATIC_4: CinemachineData = CinemachineData {
        name: "intro".to_string(),
        clip: Clip{
            frame_count: 5,
            frame_duration_millis: 200,
            image_path: "/cinematics/intro/falling_scene_anim.png".to_string(),
            start_coordinates: Vec2{x:0.0,y:0.0},
            layer_index: 100.0, 
            snd: None,
            waitInSeconds: 2.0,
        }
    };
}