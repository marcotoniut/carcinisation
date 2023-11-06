use bevy::prelude::*;
use seldom_pixel::prelude::*;

use crate::globals::GBColor;

#[derive(Component, Default)]
pub struct PxRectangle<L: PxLayer> {
    pub canvas: PxCanvas,
    pub color: GBColor,
    pub anchor: PxAnchor,
    pub layer: L,
    pub height: u32,
    pub width: u32,
}

impl<L: PxLayer> PxRectangle<L> {
    pub fn row_line_vec(&self, position: Vec2, row: &PxRectangleRow) -> Vec<IVec2> {
        let x = position.x as i32;
        let y = (position.y + row.0 as f32) as i32;
        let height = self.height as i32;
        let width = self.width as i32;
        let half_height = height / 2;
        let half_width = width / 2;

        match self.anchor {
            PxAnchor::BottomCenter => {
                vec![IVec2::new(x - half_width, y), IVec2::new(x + half_width, y)]
            }
            PxAnchor::BottomLeft => {
                vec![IVec2::new(x, y), IVec2::new(x + width, y)]
            }
            PxAnchor::BottomRight => {
                vec![IVec2::new(x - width, y), IVec2::new(x, y)]
            }
            PxAnchor::Center => {
                vec![
                    IVec2::new(x - half_width, y - half_height),
                    IVec2::new(x + half_width, y + half_height),
                ]
            }
            PxAnchor::CenterLeft => {
                vec![
                    IVec2::new(x, y - half_height),
                    IVec2::new(x + width, y + half_height),
                ]
            }
            PxAnchor::CenterRight => {
                vec![
                    IVec2::new(x - width, y - half_height),
                    IVec2::new(x, y + half_height),
                ]
            }
            PxAnchor::TopCenter => {
                vec![
                    IVec2::new(x - half_width, y - height),
                    IVec2::new(x + half_width, y - height),
                ]
            }
            PxAnchor::TopLeft => {
                vec![IVec2::new(x, y - height), IVec2::new(x + width, y - height)]
            }
            PxAnchor::TopRight => {
                vec![IVec2::new(x - width, y - height), IVec2::new(x, y - height)]
            }
            PxAnchor::Custom(v) => {
                // TODO implement
                vec![]
            }
        }
    }
}

#[derive(Component, Copy, Clone, Debug)]
pub struct PxRectangleRow(pub u32);
