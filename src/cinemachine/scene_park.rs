use bevy::prelude::*;

use crate::stage::data::{
    ContainerSpawn, DestructibleSpawn, EnemySpawn, ObjectSpawn, ObjectType, SkyboxData, StageData,
    StageStep,
};
use crate::stage::data::{
    DestructibleType, EnemyStep, EnemyType, StageActionResumeCondition, StageSpawn,
};

use crate::cinemachine::*;

use bevy::prelude::*;
use lazy_static::lazy_static;

use super::data::Clip;

lazy_static! {
    pub static ref PARK_ANIMATIC: CinemachineData = CinemachineData {
        name: "park".to_string(),
        clip: Clip {
            frame_count: 4,
            frame_duration_millis: 2000,
            image_path: "/cinematics/intro/0.png".to_string(),
            start_coordinates: Vec2::ZERO,
            music_path_o: None,
            duration: Duration::from_secs_f32(3.0),
        }
    };
}
