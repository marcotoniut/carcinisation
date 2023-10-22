use crate::cutscene::data::*;
use bevy::prelude::*;
use lazy_static::lazy_static;
use std::{sync::Arc, time::Duration};

lazy_static! {
    pub static ref CINEMATIC_INTRO_DATA: Arc<CutsceneData> = Arc::new(CutsceneData {
        name: "Intro".to_string(),
        steps: vec![
            CutsceneMusicSpawn::new("audio/music/action.ogg".to_string()).into(),
            CutsceneAnimationsSpawn::new()
                .push_spawn(CutsceneAnimationSpawn {
                    duration: Duration::from_secs_f32(0.3),
                    frame_count: 5,
                    image_path: "cinematics/intro/acrab_travelling.png".to_string(),
                    start_coordinates: Vec2::ZERO,
                    tag_o: None
                })
                .into(),
            CutsceneElapse::from_secs_f32(3.0).clear_graphics().into(),
            CutsceneAnimationsSpawn::new()
                .push_spawn(CutsceneAnimationSpawn {
                    duration: Duration::from_secs_f32(4.0),
                    frame_count: 1,
                    image_path: "cinematics/intro/asteroid_waves.png".to_string(),
                    start_coordinates: Vec2::ZERO,
                    tag_o: None
                })
                .into(),
            CutsceneElapse::from_secs_f32(2.5).clear_graphics().into(),
            CutsceneAnimationsSpawn::new()
                .push_spawn(CutsceneAnimationSpawn {
                    duration: Duration::from_secs_f32(2.0),
                    frame_count: 1,
                    image_path: "cinematics/intro/screaming_scene.png".to_string(),
                    start_coordinates: Vec2::ZERO,
                    tag_o: None
                })
                .into(),
            CutsceneElapse::from_secs_f32(2.5).clear_graphics().into(),
            CutsceneAnimationsSpawn::new()
                .push_spawn(CutsceneAnimationSpawn {
                    duration: Duration::from_secs_f32(2.0),
                    frame_count: 1,
                    image_path: "cinematics/intro/transform.png".to_string(),
                    start_coordinates: Vec2::ZERO,
                    tag_o: None
                })
                .into(),
            CutsceneElapse::from_secs_f32(2.5).clear_graphics().into(),
            CutsceneAnimationsSpawn::new()
                .push_spawn(CutsceneAnimationSpawn {
                    duration: Duration::from_secs_f32(0.2),
                    frame_count: 2,
                    image_path: "cinematics/intro/falling_scene_animated.png".to_string(),
                    start_coordinates: Vec2::ZERO,
                    tag_o: None
                })
                .into(),
            CutsceneElapse::from_secs_f32(3.0).clear_graphics().into(),
            CutsceneMusicDespawn::new().into(),
        ]
    });
}
