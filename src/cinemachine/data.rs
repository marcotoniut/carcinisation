use bevy::prelude::*;

pub struct TargetPath {
    pub move_to_target: Vec2,
    pub move_speed: f32,
}

pub struct Clip {
    pub background_path: Option<String>,
    pub foreground_path: Option<String>,
    pub start_coordinates: Vec2,
    pub target_path: Option<TargetPath>,
    pub layer_index: f32
}

pub struct CinemachineData<'a> { 
    pub name: String,
    pub start_coordinates: Vec2,
    pub default_background_filter_color: &'a str,
    pub clips: Vec<Clip>,
}
