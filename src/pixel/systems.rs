use bevy::{prelude::*, utils::HashMap};
use seldom_pixel::{
    asset::PxAsset,
    filter::PxFilterData,
    prelude::{PxAssets, PxFilter, PxFilterLayers, PxLayer, PxLine, PxLineBundle, PxSubPosition},
};

use crate::globals::GBColor;

use super::components::*;

pub fn spawn_rectangle_rows<L: PxLayer>(
    parent: &mut ChildBuilder<'_, '_, '_>,
    rectangle: &PxRectangle<L>,
    filter: &Handle<PxAsset<PxFilterData>>,
) {
    for row in 0..rectangle.height {
        let row = PxRectangleRow(row);
        parent.spawn((
            row,
            PxLineBundle::<L> {
                canvas: rectangle.canvas,
                // TODO this calculations are a bit pointless now
                line: rectangle.row_line_vec(Vec2::ZERO, &row).into(),
                filter: filter.clone(),
                layers: PxFilterLayers::single_over(rectangle.layer.clone()),
                ..Default::default()
            },
        ));
    }
}

pub fn construct_rectangle<L: PxLayer>(
    mut commands: Commands,
    mut filters: PxAssets<PxFilter>,
    query: Query<(Entity, &PxRectangle<L>), Added<PxRectangle<L>>>,
) {
    for (entity, rectangle) in query.iter() {
        commands
            .entity(entity)
            .insert(rectangle.color)
            .with_children(|parent| {
                let filter = filters.load(rectangle.color.get_filter_path());
                spawn_rectangle_rows(parent, rectangle, &filter);
            });
    }
}

pub fn update_rectangle_color<L: PxLayer>(
    mut commands: Commands,
    mut filters: PxAssets<PxFilter>,
    mut query: Query<(Entity, &PxRectangle<L>, Ref<GBColor>)>,
) {
    for (parent, rectangle, color) in query.iter_mut() {
        if color.is_changed() && !color.is_added() {
            commands
                .entity(parent)
                .despawn_descendants()
                .with_children(|parent| {
                    let filter = filters.load(color.get_filter_path());
                    spawn_rectangle_rows(parent, rectangle, &filter);
                });
        }
    }
}

pub fn update_rectangle_position<L: PxLayer>(
    mut parents_query: Query<(Entity, &PxRectangle<L>, Ref<PxSubPosition>, Ref<Children>)>,
    mut children_query: Query<(&Parent, &PxRectangleRow, &mut PxLine)>,
) {
    let mut map: HashMap<Entity, (&PxRectangle<L>, Ref<PxSubPosition>, Ref<Children>)> =
        HashMap::new();

    for (parent, rectangle, position, children) in parents_query.iter_mut() {
        map.insert(parent, (rectangle, position, children));
    }

    for (parent, row, mut line) in children_query.iter_mut() {
        if let Some((rectangle, position, children)) = map.get(&parent.get()) {
            if position.is_added() || position.is_changed() || children.is_changed() {
                line.0 = rectangle.row_line_vec(position.0, row);
            }
        }
    }
}
