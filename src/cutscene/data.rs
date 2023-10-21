use bevy::prelude::{Component, Resource, Vec2};
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct TargetPath {
    pub move_to_target: Vec2,
    pub move_speed: f32,
}

#[derive(Clone, Debug, Component)]
pub struct CutsceneAnimationSpawn {
    pub duration: Duration,
    pub frame_count: usize,
    pub image_path: String,
    pub music_path_o: Option<String>,
    pub start_coordinates: Vec2,
    pub tag: String,
}

#[derive(Clone, Debug, Resource)]
pub struct CinematicData {
    pub name: String,
    pub steps: Vec<CutsceneAnimationSpawn>,
}
