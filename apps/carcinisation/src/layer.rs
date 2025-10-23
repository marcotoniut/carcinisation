use bevy::reflect::Reflect;
use seldom_pixel::{math::Next, prelude::px_layer};
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

impl Next for MidDepth {
    const MIN: Self = MidDepth::Six;

    fn next(self) -> Option<Self> {
        use MidDepth::*;

        match self {
            Six => Some(Five),
            Five => Some(Four),
            Four => Some(Three),
            Three => Some(Two),
            Two => Some(One),
            One => Some(Zero),
            Zero => None,
        }
    }
}

impl Next for PreBackgroundDepth {
    const MIN: Self = PreBackgroundDepth::Nine;

    fn next(self) -> Option<Self> {
        use PreBackgroundDepth::*;

        match self {
            Nine => Some(Eight),
            Eight => Some(Seven),
            Seven => None,
        }
    }
}

impl Next for CutsceneLayer {
    const MIN: Self = CutsceneLayer::Background(0);

    fn next(self) -> Option<Self> {
        use CutsceneLayer::*;

        match self {
            Background(layer) if layer < u8::MAX => Some(Background(layer + 1)),
            Background(_) => Some(Middle(0)),
            Middle(layer) if layer < u8::MAX => Some(Middle(layer + 1)),
            Middle(_) => Some(Letterbox),
            Letterbox => Some(Foreground(0)),
            Foreground(layer) if layer < u8::MAX => Some(Foreground(layer + 1)),
            Foreground(_) => Some(Textbox),
            Textbox => Some(Text),
            Text => Some(Overtext(0)),
            Overtext(layer) if layer < u8::MAX => Some(Overtext(layer + 1)),
            Overtext(_) => None,
        }
    }
}
