use crate::{cutscene::data::*, letterbox::events::LetterboxMoveEvent, Layer};
use assert_assets_path::assert_assets_path;
use lazy_static::lazy_static;
use std::sync::Arc;

pub const TAG_SHIP: &str = "ship";

lazy_static! {
    pub static ref CINEMATIC_INTRO_DATA: Arc<CutsceneData> = Arc::new(
        (CutsceneData {
            name: "Intro".to_string(),
            steps: vec![
                CutsceneAct::new().move_letterbox(LetterboxMoveEvent::open()),
                CutsceneAct::new()
                    .spawn_music(CutsceneMusicSpawn::new(
                        assert_assets_path!("audio/music/action.ogg").to_string(),
                    ))
                    .spawn_animations(
                        CutsceneAnimationsSpawn::new().push_spawn(CutsceneAnimationSpawn::new(
                            assert_assets_path!("cinematics/intro/acrab_travelling.png")
                                .to_string(),
                            5,
                            0.3,
                        )),
                    )
                    .with_elapse(3.0),
                CutsceneAct::new()
                    .spawn_images(
                        CutsceneImagesSpawn::new().push_spawn(CutsceneImageSpawn::new(
                            assert_assets_path!("cinematics/intro/asteroid_waves.png").to_string(),
                        )),
                    )
                    .with_elapse(2.5),
                CutsceneAct::new()
                    .spawn_images(
                        CutsceneImagesSpawn::new().push_spawn(CutsceneImageSpawn::new(
                            assert_assets_path!("cinematics/intro/screaming_scene.png").to_string(),
                        )),
                    )
                    .with_elapse(2.5),
                CutsceneAct::new()
                    .spawn_images(
                        CutsceneImagesSpawn::new().push_spawn(CutsceneImageSpawn::new(
                            assert_assets_path!("cinematics/intro/transform.png").to_string(),
                        )),
                    )
                    .with_elapse(2.5),
                CutsceneAct::new()
                    .spawn_animations(
                        CutsceneAnimationsSpawn::new().push_spawn(
                            CutsceneAnimationSpawn::new(
                                assert_assets_path!("cinematics/intro/falling_ship.png")
                                    .to_string(),
                                2,
                                0.2,
                            )
                            .with_tag(TAG_SHIP.to_string())
                            .with_coordinates(25.0, 60.0)
                            .with_layer(Layer::CutsceneLayer(CutsceneLayer::Middle(0)))
                            .with_target_movement(TargetMovement::new(40.0, 20.0).with_speed(0.3)),
                        ),
                    )
                    .spawn_images(
                        CutsceneImagesSpawn::new().push_spawn(CutsceneImageSpawn::new(
                            assert_assets_path!("cinematics/intro/planet.png").to_string(),
                        )),
                    )
                    .with_elapse(5.0),
                CutsceneAct::new()
                    .despawn_music()
                    .move_letterbox(LetterboxMoveEvent::hide()),
            ],
        })
        .set_steps(vec![
            CutsceneAct::new().move_letterbox(LetterboxMoveEvent::open()),
            CutsceneAct::new()
                .spawn_images(
                    CutsceneImagesSpawn::new().push_spawn(CutsceneImageSpawn::new(
                        assert_assets_path!("cinematics/intro/transform.png").to_string(),
                    )),
                )
                .with_elapse(2.0),
            CutsceneAct::new().move_letterbox(LetterboxMoveEvent::hide()),
        ]),
    );
}
