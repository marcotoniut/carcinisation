use bevy::reflect::Reflect;
use seldom_pixel::prelude::px_layer;
use serde::{Deserialize, Serialize};

use crate::cutscene::data::CutsceneLayer;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Hash, PartialOrd, Ord, Reflect, Serialize)]
pub enum MidDepth {
    Six,
    Five,
    Four,
    Three,
    Two,
    One,
    Zero,
}

#[derive(Debug, Deserialize, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Reflect, Serialize)]
pub enum PreBackgroundDepth {
    Nine,
    Eight,
    Seven,
}

#[derive(Deserialize, Reflect, Serialize)]
#[px_layer]
pub enum Layer {
    Skybox,

    PreBackgroundDepth(PreBackgroundDepth),
    Background,
    MidDepth(MidDepth),

    Attack,
    #[default]
    Front,
    HudBackground,
    Hud,
    Pickups,
    UIBackground,
    UI,
    CutsceneLayer(CutsceneLayer),
    // CutsceneBackground,
    // Cutscene(u8),
    // Letterbox,
    // CutsceneText,
    Transition,
}

// impl Layer {
//     pub fn get_from_depth(entity_type: StageEntityType, depth: DepthBase) -> Self {}
// }
