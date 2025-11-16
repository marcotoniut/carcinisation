//! Compatibility bundles and helpers that approximate the pre-0.8 `seldom_pixel`
//! spawn ergonomics while using the newer component-driven API.

use super::components::*;
use crate::components::GBColor;
use crate::layer::Layer;
use bevy::{
    ecs::system::EntityCommands,
    prelude::{AssetServer, Bundle, Component, InheritedVisibility, Visibility},
};
use seldom_pixel::prelude::{
    PxAnchor, PxAnimation, PxAnimationDirection, PxAnimationDuration, PxAnimationFinishBehavior,
    PxCanvas, PxFilter, PxFilterLayers, PxFrame, PxFrameTransition, PxLayer, PxLine, PxPosition,
    PxSprite, PxText,
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

/// Minimal shim around `PxAnimation` to preserve the old bundle name.
#[derive(Bundle, Default, Clone)]
pub struct PxAnimationBundle {
    pub animation: PxAnimation,
    pub frame: PxFrame,
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
        }
    }

    pub fn animation(&self) -> &PxAnimation {
        &self.animation
    }

    pub fn animation_mut(&mut self) -> &mut PxAnimation {
        &mut self.animation
    }
}

/// Inserts a pixel rectangle of the given size and colour onto the supplied entity.
pub fn insert_rectangle(
    entity_commands: &mut EntityCommands,
    width: u32,
    height: u32,
    asset_server: &AssetServer,
    color: GBColor,
) {
    entity_commands
        .insert(PxRectangle::<Layer> {
            height,
            width,
            ..Default::default()
        })
        .insert((Visibility::Visible, InheritedVisibility::VISIBLE))
        .with_children(|p0| {
            for row in 0..height {
                let i = row as i32;
                p0.spawn((
                    PxRectangleRow(row),
                    PxLineBundle::<Layer> {
                        canvas: PxCanvas::Camera,
                        line: [(0, i).into(), (width as i32, i).into()].into(),
                        layers: PxFilterLayers::single_over(Layer::Transition),
                        filter: PxFilter(asset_server.load(color.get_filter_path())),
                        ..Default::default()
                    },
                ));
            }
        });
}
