use bevy::prelude::*;
use bevy_prototype_lyon::path::PathBuilder;

use crate::components::{CutsceneActConnection, CutsceneActNode};

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
                let mut origin_position = origin_transform.translation;
                let mut target_position = target_transform.translation;

                origin_position.z = -1.;
                target_position.z = -1.;

                // Create a new path from the origin to the target
                let mut path_builder = PathBuilder::new();
                path_builder.move_to(origin_position.truncate());
                path_builder.line_to(target_position.truncate());
                let shape = path_builder.build();

                // Update the path of the connection
                // *path = GeometryBuilder::build_as(&shape);
                commands.entity(connection_entity).insert(shape);
            }
            _ => {
                // If either the origin or target entity is not found, despawn the connection entity
                // commands.entity(connection_entity).despawn();
            }
        };
    }
}
