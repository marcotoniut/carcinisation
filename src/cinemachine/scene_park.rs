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

lazy_static! {
    pub static ref PARK_ANIMATIC: CinemachineData = CinemachineData {
        name: "park".to_string(),
        start_coordinates: Some(Vec2::new(0.0, 0.0)),
    };
}