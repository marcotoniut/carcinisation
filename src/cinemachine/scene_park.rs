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
    pub static ref PARK_ANIMATIC: CinemachineData = CinemachineData {
        name: "park".to_string(),
        start_coordinates: Vec2::new(0.0, 0.0),
        clips: make_clips()
    };
}

fn make_clips() -> Vec<Clip>{
    vec![]
}