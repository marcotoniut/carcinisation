use bevy::{
    prelude::{AssetServer, Handle, Res, Resource},
    reflect::{TypePath, TypeUuid},
};
use serde::Deserialize;
use std::{fs, str::FromStr};

#[derive(Deserialize, TypeUuid, TypePath, Clone, Debug)]
#[uuid = "c17075ed-7df0-4a51-b961-ce5270a8a934"]
pub struct StageData {
    pub name: String,
    pub background: String,
    pub skybox: String,
    pub actions: Vec<Action>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    #[serde(rename = "Movement")]
    Movement {
        coordinates: [f64; 2],
        base_speed: f64,
    },
    #[serde(rename = "Stop")]
    Stop {
        condition: String,
        max_duration: Option<f64>,
        coordinates: [f64; 2],
    },
}

#[derive(Resource)]
pub struct StageDataHandle(pub Handle<StageData>);
