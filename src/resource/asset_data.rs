use bevy::prelude::*;

use crate::stage::data::StageData;
use crate::stage::data::StageSpawn;
use crate::stage::data::StageStep;
use serde::Deserialize;

type CallStageSpawns = fn() -> Vec<StageSpawn>;
type CallStageSteps = fn() -> Vec<StageStep>;

#[derive(Deserialize, Debug, Clone)]
pub struct SkyboxData {
    pub path: String,
    pub frames: usize,
}

pub struct AssetData {
    pub name: String,
    pub background: String,
    pub skybox: SkyboxData,
    pub start_coordinates: Option<Vec2>,
    pub _get_spawns: CallStageSpawns,
    pub _get_steps: CallStageSteps,
}

impl AssetData {
    pub fn get_data(&self) -> StageData {
        let spawns = (self._get_spawns)();
        let steps = (self._get_steps)();

        let stage_data = StageData {
            name: self.name.to_string(),
            background: self.background.to_string(),
            skybox: Some(self.skybox.clone()),
            start_coordinates: self.start_coordinates,
            spawns,
            steps,
        };
        return stage_data;
    }
}

pub fn _get_spawns() -> Vec<StageSpawn> {
    return Vec::new();
}

pub fn _get_steps() -> Vec<StageStep> {
    return Vec::new();
}
