//! Convenience bundles for spawning common Carapace entity configurations.
//!
//! These bundles enforce that a rendering layer is always provided (preventing
//! silent `DefaultLayer` bugs) while keeping spawn sites concise.
//!
//! All core components (`CxSprite`, `CxText`, etc.) already use `#[require]`
//! for their mandatory companions. These bundles add the game-specific layer
//! field that `#[require]` cannot know about.

use crate::{
    animation::{
        CxAnimation, CxAnimationDirection, CxAnimationDuration, CxAnimationFinishBehavior,
    },
    filter::{CxFilter, CxFilterLayers},
    frame::{CxFrameControl, CxFrameTransition, CxFrameView},
    position::{CxAnchor, CxLayer, CxPosition},
    prelude::*,
    rect::CxFilterRect,
    sprite::CxSprite,
    text::CxText,
};

#[cfg(feature = "line")]
use crate::line::CxLine;

/// Sprite entity with an explicit rendering layer.
#[derive(Bundle, Default)]
pub struct CxSpriteBundle<L: Component + CxLayer + Default + Clone> {
    /// The sprite asset handle.
    pub sprite: CxSprite,
    /// Pixel position.
    pub position: CxPosition,
    /// Anchor point (default: BottomLeft).
    pub anchor: CxAnchor,
    /// Rendering layer (game-specific).
    pub layer: L,
    /// World or Camera space.
    pub canvas: CxRenderSpace,
    /// Visibility state.
    pub visibility: Visibility,
}

/// Text entity with an explicit rendering layer.
#[derive(Bundle, Default)]
pub struct CxTextBundle<L: Component + CxLayer + Default + Clone> {
    /// The text content + typeface handle.
    pub text: CxText,
    /// Pixel position.
    pub position: CxPosition,
    /// Anchor point.
    pub anchor: CxAnchor,
    /// Rendering layer.
    pub layer: L,
    /// World or Camera space.
    pub canvas: CxRenderSpace,
    /// Visibility state.
    pub visibility: Visibility,
}

/// Line entity with filter layers.
#[cfg(feature = "line")]
#[derive(Bundle, Default)]
pub struct CxLineBundle<L: Component + CxLayer + Default + Clone> {
    /// The line definition.
    pub line: CxLine,
    /// Filter layer set.
    pub layers: CxFilterLayers<L>,
    /// Palette filter.
    pub filter: CxFilter,
    /// World or Camera space.
    pub canvas: CxRenderSpace,
    /// Visibility state.
    pub visibility: Visibility,
}

/// Filter rect with an explicit rendering layer.
#[derive(Bundle, Default)]
pub struct CxFilterRectBundle<L: Component + CxLayer + Default + Clone> {
    /// The filter rect definition.
    pub rect: CxFilterRect,
    /// Pixel position.
    pub position: CxPosition,
    /// Anchor point.
    pub anchor: CxAnchor,
    /// World or Camera space.
    pub canvas: CxRenderSpace,
    /// Filter layer set.
    pub layers: CxFilterLayers<L>,
    /// Palette filter.
    pub filter: CxFilter,
    /// Visibility state.
    pub visibility: Visibility,
}

/// Animation bundle: groups `CxAnimation` with its frame view and control.
///
/// While `CxAnimation` already `#[require]`s `CxFrameView` and `CxFrameControl`,
/// this bundle provides the [`from_parts`](Self::from_parts) constructor for
/// setting non-default values on both components in one call.
#[derive(Bundle, Default, Clone)]
pub struct CxAnimationBundle {
    /// The animation state.
    pub animation: CxAnimation,
    /// Frame display settings (transition mode, etc.).
    pub frame: CxFrameView,
    /// Frame timing control.
    pub frame_control: CxFrameControl,
}

impl CxAnimationBundle {
    /// Construct with explicit direction, duration, finish behavior, and frame transition.
    #[must_use]
    pub fn from_parts(
        direction: CxAnimationDirection,
        duration: CxAnimationDuration,
        on_finish: CxAnimationFinishBehavior,
        frame_transition: CxFrameTransition,
    ) -> Self {
        Self {
            animation: CxAnimation {
                direction,
                duration,
                on_finish,
                ..Default::default()
            },
            frame: CxFrameView {
                transition: frame_transition,
                ..Default::default()
            },
            frame_control: CxFrameControl::default(),
        }
    }

    /// Read-only access to the animation component.
    #[must_use]
    pub fn animation(&self) -> &CxAnimation {
        &self.animation
    }

    /// Mutable access to the animation component.
    pub fn animation_mut(&mut self) -> &mut CxAnimation {
        &mut self.animation
    }
}
