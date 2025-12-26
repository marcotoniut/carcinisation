//! Compatibility bundles and helpers that approximate the pre-0.8 `seldom_pixel`
//! spawn ergonomics while using the newer component-driven API.

use bevy::prelude::{Bundle, Component, Visibility};
use seldom_pixel::prelude::{
    PxAnchor, PxAnimation, PxAnimationDirection, PxAnimationDuration, PxAnimationFinishBehavior,
    PxCanvas, PxFilter, PxFilterLayers, PxFrame, PxFrameControl, PxFrameTransition, PxLayer,
    PxLine, PxPosition, PxRect, PxSprite, PxText,
};

/// Equivalent of the legacy `PxSpriteBundle`, rebuilt on top of the 0.8 component set.
#[derive(Bundle, Default)]
pub struct PxSpriteBundle<L: Component + PxLayer + Default + Clone> {
    pub sprite: PxSprite,
    pub position: PxPosition,
    pub anchor: PxAnchor,
    pub layer: L,
    pub canvas: PxCanvas,
    pub visibility: Visibility,
}

/// Equivalent of the legacy `PxTextBundle`, rebuilt on top of the 0.8 component set.
#[derive(Bundle, Default)]
pub struct PxTextBundle<L: Component + PxLayer + Default + Clone> {
    pub text: PxText,
    pub position: PxPosition,
    pub anchor: PxAnchor,
    pub layer: L,
    pub canvas: PxCanvas,
    pub visibility: Visibility,
}

/// Equivalent of the legacy `PxLineBundle`, rebuilt on top of the 0.8 component set.
#[derive(Bundle, Default)]
pub struct PxLineBundle<L: Component + PxLayer + Default + Clone> {
    pub line: PxLine,
    pub layers: PxFilterLayers<L>,
    pub filter: PxFilter,
    pub canvas: PxCanvas,
    pub visibility: Visibility,
}

/// Convenience bundle for `PxRect` with its required components.
#[derive(Bundle, Default)]
pub struct PxRectBundle<L: Component + PxLayer + Default + Clone> {
    pub rect: PxRect,
    pub position: PxPosition,
    pub anchor: PxAnchor,
    pub canvas: PxCanvas,
    pub layers: PxFilterLayers<L>,
    pub filter: PxFilter,
    pub visibility: Visibility,
}

/// Minimal shim around `PxAnimation` to preserve the old bundle name.
#[derive(Bundle, Default, Clone)]
pub struct PxAnimationBundle {
    pub animation: PxAnimation,
    pub frame: PxFrame,
    pub frame_control: PxFrameControl,
}

impl PxAnimationBundle {
    pub fn from_parts(
        direction: PxAnimationDirection,
        duration: PxAnimationDuration,
        on_finish: PxAnimationFinishBehavior,
        frame_transition: PxFrameTransition,
    ) -> Self {
        Self {
            animation: PxAnimation {
                direction,
                duration,
                on_finish,
                ..Default::default()
            },
            frame: PxFrame {
                transition: frame_transition,
                ..Default::default()
            },
            frame_control: PxFrameControl::default(),
        }
    }

    pub fn animation(&self) -> &PxAnimation {
        &self.animation
    }

    pub fn animation_mut(&mut self) -> &mut PxAnimation {
        &mut self.animation
    }
}
