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
    pub coordinates: Vec2,
    pub tag_o: Option<String>,
}

impl CutsceneAnimationSpawn {
    pub fn new(image_path: String, frame_count: usize, secs: f32) -> Self {
        Self {
            duration: Duration::from_secs_f32(secs),
            frame_count,
            image_path,
            coordinates: Vec2::ZERO,
            tag_o: None,
        }
    }

    pub fn with_coordinates(mut self, coordinates: Vec2) -> Self {
        self.coordinates = coordinates;
        self
    }

    pub fn with_tag(mut self, tag: String) -> Self {
        self.tag_o = Some(tag);
        self
    }
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
pub struct CutsceneElapse {
    pub duration: Duration,
    pub clear_graphics: bool,
}

impl CutsceneElapse {
    pub fn new(duration: Duration) -> Self {
        Self {
            duration,
            clear_graphics: false,
        }
    }

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

#[derive(Clone, Debug)]
pub struct CutsceneAct {
    pub await_input: bool,
    pub despawn_entities: Vec<String>,
    pub elapse: Duration,
    pub music_despawn_o: Option<CutsceneMusicDespawn>,
    pub music_spawn_o: Option<CutsceneMusicSpawn>,
    pub spawn_animations_o: Option<CutsceneAnimationsSpawn>,
}

impl CutsceneAct {
    pub fn new() -> Self {
        Self {
            await_input: false,
            despawn_entities: vec![],
            elapse: Duration::ZERO,
            music_despawn_o: None,
            music_spawn_o: None,
            spawn_animations_o: None,
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

    pub fn despawn_music(mut self) -> Self {
        self.music_despawn_o = Some(CutsceneMusicDespawn {});
        self
    }

    pub fn with_elapse(mut self, secs: f32) -> Self {
        self.elapse = Duration::from_secs_f32(secs);
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
pub struct CutsceneSpriteSpawn {
    pub image_path: String,
    pub coordinates: Vec2,
    pub tag_o: Option<String>,
}

impl CutsceneSpriteSpawn {
    pub fn new(image_path: String, coordinates: Vec2) -> Self {
        Self {
            image_path,
            coordinates,
            tag_o: None,
        }
    }
}

#[derive(Clone, Debug, Resource)]
pub struct CutsceneData {
    pub name: String,
    pub steps: Vec<CutsceneAct>,
}
