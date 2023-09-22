use crate::stage::data::StageData;

use self::{asset_data::AssetData, asteroid::ASTEROID_DATA};

pub mod asteroid;
pub mod asset_data;

pub fn get_stage_data(data: AssetData) -> StageData {
    let mut skybox : Option<String> = None;
    if data.skybox.is_some() {
        skybox = Some(data.skybox.unwrap().to_string());
    }
    let spawns = (data._get_spawns)();
    let steps = (data._get_steps)();

    let stage_data = StageData{
        name: data.name.to_string(),
        background: data.background.to_string(),
        skybox,
        start_coordinates: data.start_coordinates,
        spawns,
        steps,
    };
    return stage_data;
}