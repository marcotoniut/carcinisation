use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;

use crate::components::{CutsceneActConnection, CutsceneActNode};

/// @system Redraws lines between cutscene act nodes when their transforms change.
pub fn update_cutscene_act_connections(
    mut commands: Commands,
    mut cutscene_act_connections_query: Query<(Entity, &CutsceneActConnection)>,
    cutscene_act_node_query: Query<&Transform, With<CutsceneActNode>>,
) {
    for (connection_entity, connection) in cutscene_act_connections_query.iter_mut() {
        match (
            cutscene_act_node_query.get(connection.origin),
            cutscene_act_node_query.get(connection.target),
        ) {
            (Ok(origin_transform), Ok(target_transform)) => {
                let origin_position = origin_transform.translation;
                let target_position = target_transform.translation;

                let path = ShapePath::new()
                    .move_to(origin_position.truncate())
                    .line_to(target_position.truncate());

                let shape = ShapeBuilder::with(&path)
                    .stroke((Color::WHITE, 2.0))
                    .build();
                commands.entity(connection_entity).insert((
                    shape,
                    Transform::from_xyz(0.0, 0.0, -1.0),
                    GlobalTransform::default(),
                ));
            }
            _ => {
                // If either the origin or target entity is not found, despawn the connection entity
                // commands.entity(connection_entity).despawn();
            }
        };
    }
}
