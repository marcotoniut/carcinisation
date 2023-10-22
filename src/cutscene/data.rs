use bevy::prelude::{Component, Resource, Vec2};
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct TargetPath {
    pub move_to_target: Vec2,
    pub move_speed: f32,
}

#[derive(Clone, Debug)]
pub struct CutsceneAnimationSpawn {
    pub duration: Duration,
    pub frame_count: usize,
    pub image_path: String,
    pub start_coordinates: Vec2,
    pub tag_o: Option<String>,
}

#[derive(Clone, Debug, Component)]
pub struct CutsceneAnimationsSpawn {
    pub spawns: Vec<CutsceneAnimationSpawn>,
}

impl CutsceneAnimationsSpawn {
    pub fn new() -> Self {
        Self { spawns: vec![] }
    }

    pub fn push_spawn(mut self, spawn: CutsceneAnimationSpawn) -> Self {
        self.spawns.push(spawn);
        self
    }
}

#[derive(Clone, Debug, Component)]
pub struct CutsceneAwaitInput;

impl CutsceneAwaitInput {
    pub fn new() -> Self {
        Self {}
    }
}

#[derive(Clone, Debug, Component)]
pub struct CutsceneDespawn {
    pub tag: String,
}

#[derive(Clone, Debug, Component)]
pub struct CutsceneElapse {
    pub duration: Duration,
    pub clear_graphics: bool,
}

impl CutsceneElapse {
    pub fn from_secs_f32(secs: f32) -> Self {
        Self {
            duration: Duration::from_secs_f32(secs),
            clear_graphics: false,
        }
    }

    pub fn clear_graphics(mut self) -> Self {
        self.clear_graphics = true;
        self
    }
}

#[derive(Clone, Debug, Component)]
pub struct CutsceneAct {
    pub spawn_animations_o: Option<CutsceneAnimationsSpawn>,
    pub music_despawn_o: Option<CutsceneMusicDespawn>,
    pub music_spawn_o: Option<CutsceneMusicSpawn>,
}

impl CutsceneAct {
    pub fn new() -> Self {
        Self {
            spawn_animations_o: None,
            music_despawn_o: None,
            music_spawn_o: None,
        }
    }

    pub fn spawn_animations(mut self, animations: CutsceneAnimationsSpawn) -> Self {
        self.spawn_animations_o = Some(animations);
        self
    }

    pub fn spawn_music(mut self, music: CutsceneMusicSpawn) -> Self {
        self.music_spawn_o = Some(music);
        self
    }

    pub fn despawn_music(mut self, music: CutsceneMusicDespawn) -> Self {
        self.music_despawn_o = Some(music);
        self
    }
}

#[derive(Clone, Debug, Component)]
pub struct CutsceneMusicSpawn {
    pub music_path: String,
    // TODO fade_in
}

impl CutsceneMusicSpawn {
    pub fn new(music_path: String) -> Self {
        Self { music_path }
    }
}

#[derive(Clone, Debug, Component)]
pub struct CutsceneMusicDespawn {
    // TODO fade_out
}

impl CutsceneMusicDespawn {
    pub fn new() -> Self {
        Self {}
    }
}

#[derive(Clone, Debug, Component)]
pub struct CutsceneEntityDespawn(pub String);

impl CutsceneEntityDespawn {
    pub fn new(tag: String) -> Self {
        Self(tag)
    }
}

#[derive(Clone, Debug, Component)]
pub struct CutsceneSpawn {
    pub image_path: String,
    pub start_coordinates: Vec2,
    pub tag_o: Option<String>,
}

impl CutsceneSpawn {
    pub fn new(image_path: String, start_coordinates: Vec2) -> Self {
        Self {
            image_path,
            start_coordinates,
            tag_o: None,
        }
    }
}

#[derive(Clone, Debug)]
pub enum CinematicStageStep {
    CutsceneAct(CutsceneAct),
    CutsceneAnimationsSpawn(CutsceneAnimationsSpawn),
    CutsceneAwaitInput(CutsceneAwaitInput),
    CutsceneDespawn(CutsceneDespawn),
    CutsceneMusicSpawn(CutsceneMusicSpawn),
    CutsceneMusicDespawn(CutsceneMusicDespawn),
    CutsceneElapse(CutsceneElapse),
    CutsceneSpawn(CutsceneSpawn),
}

impl From<CutsceneAct> for CinematicStageStep {
    fn from(act: CutsceneAct) -> Self {
        Self::CutsceneAct(act)
    }
}

impl From<CutsceneAnimationsSpawn> for CinematicStageStep {
    fn from(spawn: CutsceneAnimationsSpawn) -> Self {
        Self::CutsceneAnimationsSpawn(spawn)
    }
}

impl From<CutsceneAwaitInput> for CinematicStageStep {
    fn from(await_input: CutsceneAwaitInput) -> Self {
        Self::CutsceneAwaitInput(await_input)
    }
}

impl From<CutsceneDespawn> for CinematicStageStep {
    fn from(despawn: CutsceneDespawn) -> Self {
        Self::CutsceneDespawn(despawn)
    }
}

impl From<CutsceneElapse> for CinematicStageStep {
    fn from(elapse: CutsceneElapse) -> Self {
        Self::CutsceneElapse(elapse)
    }
}

impl From<CutsceneMusicSpawn> for CinematicStageStep {
    fn from(music_spawn: CutsceneMusicSpawn) -> Self {
        Self::CutsceneMusicSpawn(music_spawn)
    }
}

impl From<CutsceneMusicDespawn> for CinematicStageStep {
    fn from(music_despawn: CutsceneMusicDespawn) -> Self {
        Self::CutsceneMusicDespawn(music_despawn)
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
}
