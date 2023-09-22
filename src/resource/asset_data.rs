use bevy::prelude::*;

use crate::stage::data::StageData;
use crate::stage::data::StageSpawn;
use crate::stage::data::StageStep;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct SkyboxData {
    pub path: String,
    pub frames: usize,
}

// TODO could probably do without this struct, replica of StageData
pub struct AssetData {
    pub name: String,
    pub background: String,
    pub skybox: SkyboxData,
    pub start_coordinates: Option<Vec2>,
    pub spawns: Vec<StageSpawn>,
    pub steps: Vec<StageStep>,
}

impl AssetData {
    pub fn get_data(&self) -> StageData {
        StageData {
            name: self.name.to_string(),
            background: self.background.to_string(),
            skybox: Some(self.skybox.clone()),
            start_coordinates: self.start_coordinates,
            spawns: self.spawns.clone(),
            steps: self.steps.clone(),
        }
    }
}

pub fn _get_spawns() -> Vec<StageSpawn> {
    return Vec::new();
}

pub fn _get_steps() -> Vec<StageStep> {
    return Vec::new();
}
