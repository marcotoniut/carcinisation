use bevy::{
    prelude::{AssetServer, Handle, Res, Resource},
    reflect::{TypePath, TypeUuid},
};
use serde::Deserialize;
use std::{fs, str::FromStr};

#[derive(Deserialize, Clone, Debug)]
pub struct StageMovement {
    pub coordinates: [f64; 2],
    pub base_speed: Option<f64>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct StageStop {
    pub condition: Option<String>,
    pub max_duration: Option<f64>,
    pub coordinates: Option<[f64; 2]>,
}

#[derive(Deserialize, TypeUuid, TypePath, Clone, Debug)]
#[uuid = "c17075ed-7df0-4a51-b961-ce5270a8a934"]
pub struct StageData {
    pub title: String,
    pub background: String,
    pub movements: Vec<StageMovement>,
    pub stops: Vec<StageStop>,
}

#[derive(Resource)]
pub struct StageDataHandle(pub Handle<StageData>);
