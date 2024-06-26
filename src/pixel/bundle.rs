use super::components::*;
use crate::{globals::GBColor, Layer};
use bevy::{ecs::system::EntityCommands, prelude::BuildChildren};
use seldom_pixel::prelude::{PxAssets, PxCanvas, PxFilter, PxFilterLayers, PxLineBundle};

pub fn insert_rectangle(
    entity_commands: &mut EntityCommands,
    width: u32,
    height: u32,
    filters: &mut PxAssets<PxFilter>,
    color: GBColor,
) {
    entity_commands
        .insert(PxRectangle {
            height,
            width,
            ..default()
        })
        .with_children(|p0| {
            for row in 0..height {
                let i = row as i32;
                p0.spawn((
                    PxRectangleRow(row),
                    PxLineBundle::<Layer> {
                        canvas: PxCanvas::Camera,
                        line: [(0, i).into(), (width as i32, i).into()].into(),
                        layers: PxFilterLayers::single_over(Layer::Transition),
                        filter: filters.load(color.get_filter_path()),
                        ..default()
                    },
                ));
            }
        });
}
