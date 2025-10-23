//! Systems for constructing/updating pixel rectangles rendered via `seldom_pixel`.

use crate::components::GBColor;

use super::components::*;
use crate::pixel::{PxAssets, PxLineBundle};
use bevy::prelude::*;
use seldom_pixel::prelude::{PxFilter, PxFilterLayers, PxLayer, PxLine, PxSubPosition};
use std::collections::HashMap;

/// @system Initialises the pixel rectangle and its child row entities on add.
pub fn construct_rectangle<L: PxLayer>(
    mut commands: Commands,
    filters: PxAssets<PxFilter>,
    query: Query<(Entity, &PxRectangle<L>), Added<PxRectangle<L>>>,
) {
    for (entity, rectangle) in query.iter() {
        let filter = filters.load(rectangle.color.get_filter_path());
        commands
            .entity(entity)
            .insert(rectangle.color)
            .with_children(|parent| {
                for row in 0..rectangle.height {
                    let row = PxRectangleRow(row);
                    parent.spawn((
                        row,
                        PxLineBundle::<L> {
                            canvas: rectangle.canvas,
                            line: rectangle.row_line_vec(Vec2::ZERO, &row).into(),
                            filter: PxFilter(filter.clone()),
                            layers: PxFilterLayers::single_over(rectangle.layer.clone()),
                            ..default()
                        },
                    ));
                }
            });
    }
}

/// @system Respawns rectangle rows if the colour changes.
pub fn update_rectangle_color<L: PxLayer>(
    mut commands: Commands,
    filters: PxAssets<PxFilter>,
    mut query: Query<(Entity, &PxRectangle<L>, Ref<GBColor>)>,
) {
    for (parent_entity, rectangle, color) in query.iter_mut() {
        if color.is_changed() && !color.is_added() {
            let filter = filters.load(color.get_filter_path());
            commands
                .entity(parent_entity)
                .despawn_children()
                .with_children(|parent| {
                    for row in 0..rectangle.height {
                        let row = PxRectangleRow(row);
                        parent.spawn((
                            row,
                            PxLineBundle::<L> {
                                canvas: rectangle.canvas,
                                line: rectangle.row_line_vec(Vec2::ZERO, &row).into(),
                                filter: PxFilter(filter.clone()),
                                layers: PxFilterLayers::single_over(rectangle.layer.clone()),
                                ..default()
                            },
                        ));
                    }
                });
        }
    }
}

/// @system Updates child line endpoints when the rectangle moves.
pub fn update_rectangle_position<L: PxLayer>(
    mut parents_query: Query<(Entity, &PxRectangle<L>, Ref<PxSubPosition>, Ref<Children>)>,
    mut children_query: Query<(&ChildOf, &PxRectangleRow, &mut PxLine)>,
) {
    let mut map: HashMap<Entity, (&PxRectangle<L>, Ref<PxSubPosition>, Ref<Children>)> =
        HashMap::new();

    for (parent, rectangle, position, children) in parents_query.iter_mut() {
        map.insert(parent, (rectangle, position, children));
    }

    for (parent, row, mut line) in children_query.iter_mut() {
        if let Some((rectangle, position, children)) = map.get(&parent.parent()) {
            if position.is_added() || position.is_changed() || children.is_changed() {
                line.0 = rectangle.row_line_vec(position.0, row);
            }
        }
    }
}
