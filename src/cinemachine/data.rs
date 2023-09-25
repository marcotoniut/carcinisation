use bevy::{
    prelude::{Vec2, warn},
    reflect::{TypePath, TypeUuid, List},
};

#[derive(Clone, Debug)]
pub struct TargetPath {
    pub move_to_target: Vec2,
    pub move_speed: f32,
}

#[derive(Clone, Debug)]
pub struct Clip {
    pub frame_count: usize,
    pub frame_duration_millis: u64,
    pub image_path: String,
    pub start_coordinates: Vec2,
    pub layer_index: f32,
    pub snd: Option<String>,
    pub waitInSeconds: f32
}

#[derive(TypeUuid, TypePath, Clone, Debug)]
#[uuid = "8962be51-bbd5-42b4-95a9-269294ddf17a"]
pub struct CinemachineData { 
    pub name: String,
    pub clip: Clip,
}