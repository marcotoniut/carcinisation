use super::data::GameStep;
use bevy::prelude::*;

// TODO should default be 3?
#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct Lives(pub u8);

#[derive(Resource, Default, Clone, Copy)]
pub struct GameProgress {
    pub index: usize,
}

#[derive(Clone, Debug, Resource)]
pub struct GameData {
    pub name: String,
    pub steps: Vec<GameStep>,
}
