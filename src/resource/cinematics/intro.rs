use crate::cutscene::data::*;
use bevy::prelude::*;
use lazy_static::lazy_static;
use std::{sync::Arc, time::Duration};

lazy_static! {
    pub static ref CINEMATIC_INTRO_DATA: Arc<CinematicData> = Arc::new(CinematicData {
        name: "Intro".to_string(),
        steps: vec![
            CutsceneAnimationSpawn {
                tag: "intro".to_string(),
                frame_count: 2,
                image_path: "/cinematics/intro/bald_guy.png".to_string(),
                start_coordinates: Vec2 { x: 0.0, y: 0.0 },
                music_path_o: None,
                duration: Duration::from_secs_f32(3.0),
            },
            CutsceneAnimationSpawn {
                tag: "intro".to_string(),
                frame_count: 1,
                image_path: "/cinematics/intro/1.png".to_string(),
                start_coordinates: Vec2 { x: 0.0, y: 0.0 },
                music_path_o: None,
                duration: Duration::from_secs_f32(4.0),
            },
            CutsceneAnimationSpawn {
                tag: "intro".to_string(),
                frame_count: 1,
                image_path: "/cinematics/intro/screaming_scene.png".to_string(),
                start_coordinates: Vec2 { x: 0.0, y: 0.0 },
                music_path_o: None,
                duration: Duration::from_secs_f32(2.0),
            },
            CutsceneAnimationSpawn {
                tag: "intro".to_string(),
                frame_count: 1,
                image_path: "/cinematics/intro/transform.png".to_string(),
                start_coordinates: Vec2 { x: 0.0, y: 0.0 },
                music_path_o: None,
                duration: Duration::from_secs_f32(2.0),
            },
            CutsceneAnimationSpawn {
                tag: "intro".to_string(),
                frame_count: 5,
                image_path: "/cinematics/intro/falling_scene_anim.png".to_string(),
                start_coordinates: Vec2 { x: 0.0, y: 0.0 },
                music_path_o: None,
                duration: Duration::from_secs_f32(2.0),
            }
        ]
    });
}
