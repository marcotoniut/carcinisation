use crate::stage::data::*;
use assert_assets_path::assert_assets_path;
use bevy::prelude::*;
use lazy_static::lazy_static;
use std::sync::Arc;

lazy_static! {
    pub static ref STAGE_SPACESHIP_DATA: Arc<StageData> = StageData {
        name: "Spaceship".to_string(),
        music_path: assert_assets_path!("audio/music/stage_2.ogg").to_string(),
        background_path: assert_assets_path!("backgrounds/spaceship/background.png").to_string(),
        skybox: SkyboxData {
            path: assert_assets_path!("backgrounds/spaceship/skybox.png").to_string(),
            // TODO review
            frames: 6,
        },
        start_coordinates: Some(Vec2::new(0.0, 0.0)),
        spawns: make_spawns(),
        steps: make_steps(),
    }.into();
}

pub fn make_spawns() -> Vec<StageSpawn> {
    vec![]
}

pub fn make_steps() -> Vec<StageStep> {
    vec![]
}
