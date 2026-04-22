//! Compatibility bundles and helpers that approximate the pre-0.8 `carapace`
//! spawn ergonomics while using the newer component-driven API.

use bevy::prelude::{Bundle, Component, Visibility};
use carapace::prelude::{
    CxAnchor, CxAnimation, CxAnimationDirection, CxAnimationDuration, CxAnimationFinishBehavior,
    CxFilter, CxFilterLayers, CxFilterRect, CxFrameControl, CxFrameTransition, CxFrameView,
    CxLayer, CxLine, CxPosition, CxRenderSpace, CxSprite, CxText,
};

/// Equivalent of the legacy `CxSpriteBundle`, rebuilt on top of the 0.8 component set.
#[derive(Bundle, Default)]
pub struct CxSpriteBundle<L: Component + CxLayer + Default + Clone> {
    pub sprite: CxSprite,
    pub position: CxPosition,
    pub anchor: CxAnchor,
    pub layer: L,
    pub canvas: CxRenderSpace,
    pub visibility: Visibility,
}

/// Equivalent of the legacy `CxTextBundle`, rebuilt on top of the 0.8 component set.
#[derive(Bundle, Default)]
pub struct CxTextBundle<L: Component + CxLayer + Default + Clone> {
    pub text: CxText,
    pub position: CxPosition,
    pub anchor: CxAnchor,
    pub layer: L,
    pub canvas: CxRenderSpace,
    pub visibility: Visibility,
}

/// Equivalent of the legacy `CxLineBundle`, rebuilt on top of the 0.8 component set.
#[derive(Bundle, Default)]
pub struct CxLineBundle<L: Component + CxLayer + Default + Clone> {
    pub line: CxLine,
    pub layers: CxFilterLayers<L>,
    pub filter: CxFilter,
    pub canvas: CxRenderSpace,
    pub visibility: Visibility,
}

/// Convenience bundle for `CxFilterRect` with its required components.
#[derive(Bundle, Default)]
pub struct CxFilterRectBundle<L: Component + CxLayer + Default + Clone> {
    pub rect: CxFilterRect,
    pub position: CxPosition,
    pub anchor: CxAnchor,
    pub canvas: CxRenderSpace,
    pub layers: CxFilterLayers<L>,
    pub filter: CxFilter,
    pub visibility: Visibility,
}

/// Minimal shim around `CxAnimation` to preserve the old bundle name.
#[derive(Bundle, Default, Clone)]
pub struct CxAnimationBundle {
    pub animation: CxAnimation,
    pub frame: CxFrameView,
    pub frame_control: CxFrameControl,
}

impl CxAnimationBundle {
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

    pub fn animation(&self) -> &CxAnimation {
        &self.animation
    }

    pub fn animation_mut(&mut self) -> &mut CxAnimation {
        &mut self.animation
    }
}
