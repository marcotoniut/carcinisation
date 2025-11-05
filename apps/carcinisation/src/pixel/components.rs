//! Pixel rectangle component definitions.

use crate::components::GBColor;
use bevy::prelude::*;
use seldom_pixel::prelude::*;

#[derive(Component, Default)]
/// Describes a rectangular pixel region to render using ``seldom_pixel``.
pub struct PxRectangle<L: PxLayer> {
    pub canvas: PxCanvas,
    pub color: GBColor,
    pub anchor: PxAnchor,
    pub layer: L,
    pub height: u32,
    pub width: u32,
}

impl<L: PxLayer> PxRectangle<L> {
    /// Produces the endpoints used to draw a horizontal row of the rectangle.
    pub fn row_line_vec(&self, position: Vec2, row: &PxRectangleRow) -> Vec<IVec2> {
        let base_x = position.x as i32;
        let base_y = position.y as i32;
        let width = i32::try_from(self.width).unwrap_or(i32::MAX);
        let height = i32::try_from(self.height).unwrap_or(i32::MAX);

        let anchor_offset_x = match self.anchor {
            PxAnchor::BottomLeft | PxAnchor::CenterLeft | PxAnchor::TopLeft => 0,
            PxAnchor::BottomCenter | PxAnchor::Center | PxAnchor::TopCenter => width / 2,
            PxAnchor::BottomRight | PxAnchor::CenterRight | PxAnchor::TopRight => width,
            PxAnchor::Custom(v) => (v.x * width as f32) as i32,
        };

        let anchor_offset_y = match self.anchor {
            PxAnchor::BottomLeft | PxAnchor::BottomCenter | PxAnchor::BottomRight => 0,
            PxAnchor::CenterLeft | PxAnchor::Center | PxAnchor::CenterRight => height / 2,
            PxAnchor::TopLeft | PxAnchor::TopCenter | PxAnchor::TopRight => height,
            PxAnchor::Custom(v) => (v.y * height as f32) as i32,
        };

        let start_x = base_x.saturating_sub(anchor_offset_x);
        let start_y = base_y.saturating_sub(anchor_offset_y);
        let row_delta = i32::try_from(row.0).unwrap_or(i32::MAX);
        let row_y = start_y.saturating_add(row_delta);
        let end_x = start_x.saturating_add(width);

        vec![IVec2::new(start_x, row_y), IVec2::new(end_x, row_y)]
    }
}

#[derive(Component, Copy, Clone, Debug)]
/// Component tagging child entities for each rectangle row.
pub struct PxRectangleRow(pub u32);
