use crate::stage::data::StageData;

use self::{asset_data::AssetData, asteroid::ASTEROID_DATA};

pub mod asteroid;
pub mod asset_data;

pub fn get_stage_data(data: AssetData) -> StageData {


    let mut skybox = String::new();
    if data.skybox.is_some() {
        skybox = data.skybox.unwrap().to_string();
    }
    let mut stage_data = StageData{
        name: data.name.to_string(),
        background: data.background.to_string(),
        skybox: Some(skybox),
        start_coordinates: data.start_coordinates,
        spawns: data.spawns,
        steps: data.steps,
    };
    return stage_data;
}