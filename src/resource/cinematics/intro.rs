use crate::cutscene::data::*;
use lazy_static::lazy_static;
use std::sync::Arc;

lazy_static! {
    pub static ref CINEMATIC_INTRO_DATA: Arc<CutsceneData> = Arc::new(CutsceneData {
        name: "Intro".to_string(),
        steps: vec![
            CutsceneAct::new()
                .spawn_music(CutsceneMusicSpawn::new(
                    "audio/music/action.ogg".to_string()
                ))
                .spawn_animations(CutsceneAnimationsSpawn::new().push_spawn(
                    CutsceneAnimationSpawn::new(
                        "cinematics/intro/acrab_travelling.png".to_string(),
                        5,
                        0.3
                    )
                ))
                .with_elapse(3.0),
            CutsceneAct::new()
                .spawn_animations(CutsceneAnimationsSpawn::new().push_spawn(
                    CutsceneAnimationSpawn::new(
                        "cinematics/intro/asteroid_waves.png".to_string(),
                        1,
                        4.0
                    )
                ))
                .with_elapse(2.5),
            CutsceneAct::new()
                .spawn_animations(CutsceneAnimationsSpawn::new().push_spawn(
                    CutsceneAnimationSpawn::new(
                        "cinematics/intro/screaming_scene.png".to_string(),
                        1,
                        2.0
                    )
                ))
                .with_elapse(2.5),
            CutsceneAct::new()
                .spawn_animations(CutsceneAnimationsSpawn::new().push_spawn(
                    CutsceneAnimationSpawn::new(
                        "cinematics/intro/transform.png".to_string(),
                        1,
                        2.0
                    )
                ))
                .with_elapse(2.5),
            CutsceneAct::new()
                .spawn_animations(CutsceneAnimationsSpawn::new().push_spawn(
                    CutsceneAnimationSpawn::new(
                        "cinematics/intro/falling_scene_animated.png".to_string(),
                        2,
                        0.2
                    )
                ))
                .with_elapse(3.0)
                .despawn_music(),
            CutsceneAct::new().despawn_music()
        ]
    });
}
