use bevy::{prelude::*, utils::HashMap};
use seldom_pixel::prelude::{PxFilterLayers, PxLayer, PxLine, PxLineBundle, PxSubPosition};

use super::components::*;

pub fn construct_rectangle<L: PxLayer>(
    mut commands: Commands,
    query: Query<(Entity, &PxRectangle<L>), Added<PxRectangle<L>>>,
) {
    for (entity, rectangle) in query.iter() {
        commands.entity(entity).with_children(|parent| {
            for row in 0..rectangle.height {
                let i = row as i32;
                parent.spawn((
                    PxRectangleRow(row),
                    PxLineBundle::<L> {
                        canvas: rectangle.canvas,
                        line: [(0, i).into(), (rectangle.width as i32, i).into()].into(),
                        filter: rectangle.filter.clone(),
                        layers: PxFilterLayers::single_over(rectangle.layer.clone()),
                        ..Default::default()
                    },
                ));
            }
        });
    }
}

pub fn update_rectangle_position<L: PxLayer>(
    mut parents_query: Query<(Entity, &PxRectangle<L>, &PxSubPosition)>,
    mut children_query: Query<(&Parent, &PxRectangleRow, &mut PxLine)>,
) {
    let mut map: HashMap<Entity, (&PxRectangle<L>, &PxSubPosition)> = HashMap::new();

    for (parent, rectangle, position) in parents_query.iter_mut() {
        map.insert(parent, (rectangle, position));
    }

    for (parent, row, mut line) in children_query.iter_mut() {
        if let Some((rect, position)) = map.get(&parent.get()) {
            let v = position.0;
            let x = v.x as i32;
            let y = (v.y + row.0 as f32) as i32;
            line.0 = vec![IVec2::new(x, y), IVec2::new(x + rect.width as i32, y)];
        }
    }
}
