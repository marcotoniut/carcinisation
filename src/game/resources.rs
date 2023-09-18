use bevy::{
    prelude::{AssetServer, Assets, Handle, Res, Resource, Vec2},
    reflect::{TypePath, TypeUuid},
};
use serde::Deserialize;
use std::{fs, str::FromStr, time::Duration};

#[derive(Deserialize, TypeUuid, TypePath, Clone, Debug)]
#[uuid = "c17075ed-7df0-4a51-b961-ce5270a8a934"]
pub struct StageData {
    pub name: String,
    pub background: String,
    pub skybox: Option<String>,
    pub actions: Vec<StageAction>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type")]
pub enum StageAction {
    #[serde(rename = "Movement")]
    Movement { coordinates: Vec2, base_speed: f32 },
    #[serde(rename = "Stop")]
    Stop {
        condition: String,
        max_duration: Option<u64>,
    },
}

#[derive(Resource)]
pub struct StageDataHandle(pub Handle<StageData>);

// TODO
// impl StageDataHandle {
//     pub fn get_action_by_index<'a>(
//         &self,
//         assets_stage_data: &Res<'a, Assets<StageData>>,
//         step: usize,
//     ) -> Option<&'a StageAction> {
//         if let Some(stage) = assets_stage_data.get(&self.0) {
//             let x = stage.actions.get(step);
//             x
//         } else {
//             None
//         }
//     }
// }
