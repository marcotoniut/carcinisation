use std::ptr::eq;

use bevy::prelude::*;

use crate::stage::data::{StageSpawn, StageStep};

type Call = fn();

pub struct AssetData<'a> {
    pub name: &'a str,
    pub background: &'a str,
    pub skybox: Option<&'a str>,
    pub start_coordinates: Option<Vec2>,
    pub spawns: Vec<StageSpawn>,
    pub steps: Vec<StageStep>,
    pub _set_steps: Call
}

pub static TEMPLATE_DATA: AssetData<'static> = AssetData {
    name: "Template",
    background: "backgrounds/main_menu/blank.png",
    skybox: None,
    start_coordinates: None,
    spawns: vec![],//Vec::new(),
    steps: vec![],
    _set_steps: _set_steps
};

pub fn _set_steps(){
}

impl AssetData<'static> {
    pub fn load(&self){
        (&self._set_steps)();
    }
}