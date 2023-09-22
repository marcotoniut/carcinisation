use std::ptr::eq;

use bevy::prelude::*;

use crate::stage::data::StageSpawn;
use crate::stage::data::StageStep;

type CallStageSpawns = fn() -> Vec<StageSpawn>;
type CallStageSteps = fn() -> Vec<StageStep>;

pub struct AssetData<'a> {
    pub name: &'a str,
    pub background: &'a str,
    pub skybox: Option<&'a str>,
    pub start_coordinates: Option<Vec2>,
    pub _get_spawns: CallStageSpawns,
    pub _get_steps: CallStageSteps
}

pub static TEMPLATE_DATA: AssetData<'static> = AssetData {
    name: "Template",
    background: "backgrounds/main_menu/blank.png",
    skybox: None,
    start_coordinates: None,
    _get_spawns,
    _get_steps
};

pub fn _get_spawns() -> Vec<StageSpawn> {
    return Vec::new();
}

pub fn _get_steps() -> Vec<StageStep> {
    return Vec::new();
}