//! Rendering layer enum and sub-layer types for all game modes.
//!
//! Each game mode owns a sub-layer enum. The [`Layer`] enum composes them
//! into a single sorted ordering for Carapace's render pipeline.

use bevy::prelude::*;
use carapace::{math::Next, prelude::px_layer};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// ORS sub-layers
// ---------------------------------------------------------------------------

/// Sub-layer depth for flamethrower particles. Higher = renders on top.
#[derive(
    Clone, Debug, Deserialize, Eq, PartialEq, Hash, PartialOrd, Ord, Reflect, Serialize, Copy,
)]
pub struct FlameDepth(pub u8);

impl Next for FlameDepth {
    const MIN: Self = FlameDepth(0);

    fn next(self) -> Option<Self> {
        if self.0 < 15 {
            Some(FlameDepth(self.0 + 1))
        } else {
            None
        }
    }
}

/// Depth slices for stage props in the on-rails shooter.
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

impl Next for MidDepth {
    const MIN: Self = MidDepth::Six;

    fn next(self) -> Option<Self> {
        use MidDepth::{Five, Four, One, Six, Three, Two, Zero};
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

/// Parallax planes behind the main background.
#[derive(Debug, Deserialize, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Reflect, Serialize)]
pub enum PreBackgroundDepth {
    Nine,
    Eight,
    Seven,
}

impl Next for PreBackgroundDepth {
    const MIN: Self = PreBackgroundDepth::Nine;

    fn next(self) -> Option<Self> {
        use PreBackgroundDepth::{Eight, Nine, Seven};
        match self {
            Nine => Some(Eight),
            Eight => Some(Seven),
            Seven => None,
        }
    }
}

/// On-rails shooter layer ordering.
#[derive(
    Clone, Debug, Default, Deserialize, Eq, PartialEq, Hash, PartialOrd, Ord, Reflect, Serialize,
)]
pub enum OrsLayer {
    /// Granular parallax planes behind the main background.
    PreBackgroundDepth(PreBackgroundDepth),
    /// Primary stage background (tiles, scrolling vistas).
    Background,
    /// Multiple depth slices for stage props.
    MidDepth(MidDepth),
    /// Projectiles/effects above props but below the player sprite.
    Attack,
    /// Per-particle flame ordering.
    FlameSegment(FlameDepth),
    /// Default gameplay plane (player + most enemies).
    #[default]
    Front,
    /// HUD-bound effects behind the HUD background.
    HudUnderlay,
    /// Backdrop strip for in-stage HUD widgets.
    HudBackground,
    /// HUD elements (text, crosshair) during gameplay.
    Hud,
    /// Collectible items above HUD but below menus.
    Pickups,
}

impl Next for OrsLayer {
    const MIN: Self = OrsLayer::PreBackgroundDepth(PreBackgroundDepth::Nine);

    fn next(self) -> Option<Self> {
        match self {
            Self::PreBackgroundDepth(d) => match d.next() {
                Some(d) => Some(Self::PreBackgroundDepth(d)),
                None => Some(Self::Background),
            },
            Self::Background => Some(Self::MidDepth(MidDepth::MIN)),
            Self::MidDepth(d) => match d.next() {
                Some(d) => Some(Self::MidDepth(d)),
                None => Some(Self::Attack),
            },
            Self::Attack => Some(Self::FlameSegment(FlameDepth::MIN)),
            Self::FlameSegment(d) => match d.next() {
                Some(d) => Some(Self::FlameSegment(d)),
                None => Some(Self::Front),
            },
            Self::Front => Some(Self::HudUnderlay),
            Self::HudUnderlay => Some(Self::HudBackground),
            Self::HudBackground => Some(Self::Hud),
            Self::Hud => Some(Self::Pickups),
            Self::Pickups => None,
        }
    }
}

// ---------------------------------------------------------------------------
// FPS sub-layers
// ---------------------------------------------------------------------------

/// Sub-layer ordering for first-person rendering.
#[derive(
    Clone, Debug, Deserialize, Eq, PartialEq, Hash, PartialOrd, Ord, Reflect, Serialize, Default,
)]
pub enum FpsLayer {
    /// The raycasted 3D view (walls, floor, ceiling).
    #[default]
    View,
    /// Billboards (enemies, pickups, projectiles).
    Billboards,
    /// HUD elements (crosshair, health bar).
    Hud,
}

impl Next for FpsLayer {
    const MIN: Self = FpsLayer::View;

    fn next(self) -> Option<Self> {
        match self {
            Self::View => Some(Self::Billboards),
            Self::Billboards => Some(Self::Hud),
            Self::Hud => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Cutscene sub-layers
// ---------------------------------------------------------------------------

/// Layer stack for cinematic sequences.
#[derive(Clone, Debug, Deserialize, PartialEq, PartialOrd, Eq, Ord, Reflect, Serialize)]
pub enum CutsceneLayer {
    Background(u8),
    Middle(u8),
    Letterbox,
    Foreground(u8),
    Textbox,
    Text,
    Overtext(u8),
}

impl Default for CutsceneLayer {
    fn default() -> Self {
        Self::Background(0)
    }
}

impl Next for CutsceneLayer {
    const MIN: Self = CutsceneLayer::Background(0);

    fn next(self) -> Option<Self> {
        use CutsceneLayer::{Background, Foreground, Letterbox, Middle, Overtext, Text, Textbox};
        match self {
            Background(n) if n < u8::MAX => Some(Background(n + 1)),
            Background(_) => Some(Middle(0)),
            Middle(n) if n < u8::MAX => Some(Middle(n + 1)),
            Middle(_) => Some(Letterbox),
            Letterbox => Some(Foreground(0)),
            Foreground(n) if n < u8::MAX => Some(Foreground(n + 1)),
            Foreground(_) => Some(Textbox),
            Textbox => Some(Text),
            Text => Some(Overtext(0)),
            Overtext(n) if n < u8::MAX => Some(Overtext(n + 1)),
            Overtext(_) => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Menu sub-layers
// ---------------------------------------------------------------------------

/// Menu/UI layer ordering.
#[derive(
    Clone, Debug, Deserialize, Eq, PartialEq, Hash, PartialOrd, Ord, Reflect, Serialize, Default,
)]
pub enum MenuLayer {
    /// Menu/card backgrounds.
    #[default]
    Background,
    /// Menu text/icons.
    Foreground,
}

impl Next for MenuLayer {
    const MIN: Self = MenuLayer::Background;

    fn next(self) -> Option<Self> {
        match self {
            Self::Background => Some(Self::Foreground),
            Self::Foreground => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Shared sub-layers
// ---------------------------------------------------------------------------

/// Layers shared across all modes.
#[derive(
    Clone, Debug, Deserialize, Eq, PartialEq, Hash, PartialOrd, Ord, Reflect, Serialize, Default,
)]
pub enum SharedLayer {
    /// Static sky gradients or far parallax art.
    #[default]
    Skybox,
    /// Full-screen transitions (wipes/fades) that cover everything.
    Transition,
}

impl Next for SharedLayer {
    const MIN: Self = SharedLayer::Skybox;

    fn next(self) -> Option<Self> {
        match self {
            Self::Skybox => Some(Self::Transition),
            Self::Transition => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Composed Layer enum
// ---------------------------------------------------------------------------

#[derive(Deserialize, Reflect, Serialize)]
#[px_layer]
/// Top-level rendering layer. Later variants render on top.
pub enum Layer {
    /// Shared: sky, backgrounds.
    Shared(SharedLayer),
    /// On-rails shooter gameplay layers.
    Ors(OrsLayer),
    /// First-person raycaster layers.
    Fps(FpsLayer),
    /// Cutscene cinematic layers.
    Cutscene(CutsceneLayer),
    /// Menu/UI layers.
    Menu(MenuLayer),
    /// Full-screen transitions — always on top.
    #[default]
    Transition,
}
