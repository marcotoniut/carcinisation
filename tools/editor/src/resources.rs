use std::time::Duration;

use carcinisation::{
    cutscene::data::CutsceneData,
    stage::{components::placement::Depth, data::StageData},
};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Reflect, Resource)]
pub struct CutsceneAssetHandle {
    pub handle: Handle<CutsceneData>,
    pub path: String,
}

#[derive(Debug, Reflect, Resource)]
pub struct StageAssetHandle {
    pub handle: Handle<StageData>,
    pub path: String,
}

#[derive(Debug, Reflect, Resource, Deserialize, Serialize)]
#[reflect(Resource)]
pub struct StageControlsUI {
    pub ElapsedDuration: Duration,

    pub ElapsedPath: bool,

    pub Skybox: bool,
    pub Background: bool,

    pub Nine: bool,
    pub Eight: bool,
    pub Seven: bool,
    pub Six: bool,
    pub Five: bool,
    pub Four: bool,
    pub Three: bool,
    pub Two: bool,
    pub One: bool,
    pub Zero: bool,
}

impl Default for StageControlsUI {
    fn default() -> Self {
        StageControlsUI {
            ElapsedPath: true,
            ElapsedDuration: Duration::from_secs(999),
            Skybox: true,
            Background: true,
            Nine: true,
            Eight: true,
            Seven: true,
            Six: true,
            Five: true,
            Four: true,
            Three: true,
            Two: true,
            One: true,
            Zero: true,
        }
    }
}

impl StageControlsUI {
    pub fn path_is_visible(&self) -> bool {
        self.ElapsedPath
    }

    pub fn background_is_visible(&self) -> bool {
        self.Background
    }

    pub fn skybox_is_visible(&self) -> bool {
        self.Skybox
    }

    pub fn depth_is_visible(&self, depth: Depth) -> bool {
        match depth {
            Depth::Nine => self.Nine,
            Depth::Eight => self.Eight,
            Depth::Seven => self.Seven,
            Depth::Six => self.Six,
            Depth::Five => self.Five,
            Depth::Four => self.Four,
            Depth::Three => self.Three,
            Depth::Two => self.Two,
            Depth::One => self.One,
            Depth::Zero => self.Zero,
        }
    }
}
