use bevy::prelude::*;
use seldom_pixel::prelude::PxSubPosition;

use crate::{
    stage::{
        components::{Collision, Health, SpawnDrop},
        data::{
            DestructibleSpawn, DestructibleType, EnemySpawn, EnemyType, PowerupSpawn, PowerupType,
            StageSpawn,
        },
        enemy::components::{
            Enemy, EnemyMosquito, EnemyMosquitoAttacking, ENEMY_MOSQUITO_BASE_HEALTH,
            ENEMY_MOSQUITO_RADIUS,
        },
        events::StageSpawnTrigger,
    },
    systems::camera::CameraPos,
};

pub fn read_stage_spawn_trigger(
    mut commands: Commands,
    mut event_reader: EventReader<StageSpawnTrigger>,
    camera_query: Query<&PxSubPosition, With<CameraPos>>,
) {
    let camera_pos = camera_query.get_single().unwrap();

    for event in event_reader.iter() {
        match &event.spawn {
            StageSpawn::Enemy(enemy_spawn) => spawn_enemy(&mut commands, &camera_pos, enemy_spawn),
            StageSpawn::Destructible(DestructibleSpawn {
                destructible_type,
                coordinates,
                elapsed,
                ..
            }) => match destructible_type {
                DestructibleType::Lamp => {}
                DestructibleType::Plant => {}
                DestructibleType::Window => {}
            },
            StageSpawn::Powerup(PowerupSpawn {
                powerup_type,
                coordinates,
                elapsed,
            }) => match powerup_type {
                PowerupType::BigHealthpack => {}
                PowerupType::SmallHealthpack => {}
            },
        }
    }
}

pub fn spawn_enemy(commands: &mut Commands, camera_pos: &PxSubPosition, enemy_spawn: &EnemySpawn) {
    let EnemySpawn {
        enemy_type,
        coordinates,
        base_speed,
        steps,
        contains,
        ..
    } = enemy_spawn;
    match enemy_type {
        EnemyType::Mosquito => {
            let entity = commands
                .spawn((
                    Name::new("EnemyMosquito"),
                    Enemy {},
                    EnemyMosquito {
                        base_speed: *base_speed,
                        steps: steps.clone(),
                    },
                    EnemyMosquitoAttacking { attack: None },
                    PxSubPosition::from(*coordinates + camera_pos.0),
                    Collision::Circle(ENEMY_MOSQUITO_RADIUS),
                    Health(ENEMY_MOSQUITO_BASE_HEALTH),
                ))
                .id();

            if let Some(contains) = contains {
                commands.entity(entity).insert(SpawnDrop(*contains.clone()));
            }
        }
        EnemyType::Kyle => {}
        EnemyType::Marauder => {}
        EnemyType::Spidey => {}
        EnemyType::Spidomonsta => {}
        EnemyType::Tardigrade => {}
    }
}
