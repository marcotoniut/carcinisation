use bevy::reflect::Reflect;
use seldom_pixel::{math::Next, prelude::px_layer};
use serde::{Deserialize, Serialize};

use crate::cutscene::data::CutsceneLayer;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Hash, PartialOrd, Ord, Reflect, Serialize)]
pub enum MidDepth {
    /// Furthest mid-plane (distant buildings, scenery silhouettes).
    Six,
    /// Slow parallax plane 2.
    Five,
    /// Slow parallax plane 3.
    Four,
    /// Mid-range props/sprites behind the player.
    Three,
    /// Foreground props that should still sit behind the player.
    Two,
    /// Slightly in front of the player (e.g. large enemies).
    One,
    /// Closest mid-depth plane before gameplay sprites.
    Zero,
}

#[derive(Debug, Deserialize, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Reflect, Serialize)]
pub enum PreBackgroundDepth {
    /// Far-away sky gradient or static wallpaper.
    Nine,
    /// Secondary skyboxes or slow clouds.
    Eight,
    /// Mountains/horizon elements still behind the main background.
    Seven,
}

#[derive(Deserialize, Reflect, Serialize)]
#[px_layer]
/// Rendering layers for seldom_pixel content. Entries later in the enum render on
/// top of earlier ones, so tiers are grouped roughly as:
pub enum Layer {
    /// Static sky gradients or far parallax art.
    Skybox,

    /// Granular parallax planes that sit behind the main background.
    PreBackgroundDepth(PreBackgroundDepth),
    /// Primary stage background (tiles, scrolling vistas).
    Background,
    /// Multiple depth slices for stage props (see `MidDepth`).
    MidDepth(MidDepth),

    /// Projectiles/effects that must sit above props but below the player sprite.
    Attack,
    /// Default gameplay plane (player + most enemies).
    #[default]
    Front,
    /// HUD-bound effects that should sit behind the HUD background.
    HudUnderlay,
    /// Backdrop strip for in-stage HUD widgets.
    HudBackground,
    /// HUD elements (text, crosshair) rendered during gameplay.
    Hud,
    /// Collectible items that should hover above HUD but below menus.
    Pickups,
    /// Menu/card backgrounds.
    UIBackground,
    /// Menu text/icons that must sit above everything else.
    UI,
    /// Mirrors the RON-defined layering for cutscenes (see `CutsceneLayer` docs for detail).
    CutsceneLayer(CutsceneLayer),
    /// Full-screen transitions (wipes/fades) that cover everything.
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
