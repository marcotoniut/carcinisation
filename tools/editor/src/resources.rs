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
#[serde(rename_all = "PascalCase")]
pub struct StageControlsUI {
    pub elapsed_duration: Duration,

    pub elapsed_path: bool,

    pub skybox: bool,
    pub background: bool,

    pub nine: bool,
    pub eight: bool,
    pub seven: bool,
    pub six: bool,
    pub five: bool,
    pub four: bool,
    pub three: bool,
    pub two: bool,
    pub one: bool,
    pub zero: bool,
}

impl Default for StageControlsUI {
    fn default() -> Self {
        StageControlsUI {
            elapsed_path: true,
            elapsed_duration: Duration::from_secs(999),
            skybox: true,
            background: true,
            nine: true,
            eight: true,
            seven: true,
            six: true,
            five: true,
            four: true,
            three: true,
            two: true,
            one: true,
            zero: true,
        }
    }
}

impl StageControlsUI {
    pub fn path_is_visible(&self) -> bool {
        self.elapsed_path
    }

    pub fn background_is_visible(&self) -> bool {
        self.background
    }

    pub fn skybox_is_visible(&self) -> bool {
        self.skybox
    }

    pub fn depth_is_visible(&self, depth: Depth) -> bool {
        match depth {
            Depth::Nine => self.nine,
            Depth::Eight => self.eight,
            Depth::Seven => self.seven,
            Depth::Six => self.six,
            Depth::Five => self.five,
            Depth::Four => self.four,
            Depth::Three => self.three,
            Depth::Two => self.two,
            Depth::One => self.one,
            Depth::Zero => self.zero,
        }
    }
}
