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
    pub tag_o: Option<String>,
}

#[derive(Clone, Debug, Component)]
pub struct CutsceneAwaitInput;

#[derive(Clone, Debug, Component)]
pub struct CutsceneDespawn {
    pub tag: String,
}

#[derive(Clone, Debug, Component)]
pub struct CutsceneElapse(pub Duration);

#[derive(Clone, Debug, Component)]
pub struct CutsceneMusicSpawn {
    pub music_path: String,
    // TODO fade_in
}

#[derive(Clone, Debug, Component)]
pub struct CutsceneMusicDespawn {
    // TODO fade_out
}

#[derive(Clone, Debug, Component)]
pub struct CutsceneSpawn {
    pub image_path: String,
    pub music_path_o: Option<String>,
    pub start_coordinates: Vec2,
    pub tag_o: Option<String>,
}

#[derive(Clone, Debug)]
pub enum CinematicStageStep {
    CutsceneAnimationSpawn(CutsceneAnimationSpawn),
    CutsceneAwaitInput(CutsceneAwaitInput),
    CutsceneDespawn(CutsceneDespawn),
    CutsceneMusicSpawn(CutsceneMusicSpawn),
    CutsceneMusicDespawn(CutsceneMusicDespawn),
    CutsceneElapse(CutsceneElapse),
    CutsceneSpawn(CutsceneSpawn),
}

impl From<CutsceneAnimationSpawn> for CinematicStageStep {
    fn from(spawn: CutsceneAnimationSpawn) -> Self {
        Self::CutsceneAnimationSpawn(spawn)
    }
}

impl From<CutsceneDespawn> for CinematicStageStep {
    fn from(despawn: CutsceneDespawn) -> Self {
        Self::CutsceneDespawn(despawn)
    }
}

impl From<CutsceneMusicSpawn> for CinematicStageStep {
    fn from(music_spawn: CutsceneMusicSpawn) -> Self {
        Self::CutsceneMusicSpawn(music_spawn)
    }
}

impl From<CutsceneElapse> for CinematicStageStep {
    fn from(elapse: CutsceneElapse) -> Self {
        Self::CutsceneElapse(elapse)
    }
}

impl From<CutsceneSpawn> for CinematicStageStep {
    fn from(spawn: CutsceneSpawn) -> Self {
        Self::CutsceneSpawn(spawn)
    }
}

#[derive(Clone, Debug, Resource)]
pub struct CinematicData {
    pub name: String,
    pub steps: Vec<CinematicStageStep>,
    pub music_path_o: Option<String>,
}
