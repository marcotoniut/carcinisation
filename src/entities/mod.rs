use bevy::prelude::*;

use self::entities::SpawnableEntity;

pub mod entities;

// pub fn setup(mut commands: Commands) {
//     commands.spawn(
//         bundle: SpawnableEntity {
//             asset_path: "sprites/star.png".to_string(),
//             animation_frames: 1,
//             health: 10,
//             damage: 10,
//             engament_type: SpawnableType::RANGED
//         },
//     );
// }

// pub fn spawn_random(entity_query: Query<&SpawnableEntity>) {
//     for (entity: &SpawnableEntity) in entity_query.iter() {
//         println!("data: {}", entity.asset_path);
//         //spawn_Entity(commands, window_query, asset_server, entity)
//     }
// }

// pub fn spawn_Entity(
//     mut commands: Commands,
//     window_query: Query<&Window, With<PrimaryWindow>>,
//     asset_server: Res<AssetServer>,
//     entity: &SpawnableEntity
// ){
//     let window: &Window = window_query.get_single().unwrap();

//     commands.spawn(
//         bundle: (
//             SpriteBundle {
//                 transform: Transform::from_xyz(x: window.width() / 2, y: window.height() / 2, z: 0),
//                 texture: asset_server.load(path: entity.asset_path)
//             }
//         ),
//     );
// }