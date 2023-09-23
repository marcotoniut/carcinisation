use bevy::{prelude::*, sprite};
use seldom_pixel::{
    prelude::{PxAnchor, PxAssets, PxSubPosition},
    sprite::{PxSprite, PxSpriteBundle},
};

use crate::{
    stage::{
        components::{Collision, Health, Object, SpawnDrop},
        data::{
            DestructibleSpawn, DestructibleType, EnemySpawn, EnemyType, ObjectSpawn, ObjectType,
            PowerupSpawn, PowerupType, StageSpawn,
        },
        enemy::components::{
            Enemy, EnemyMosquito, EnemyMosquitoAttacking, ENEMY_MOSQUITO_BASE_HEALTH,
            ENEMY_MOSQUITO_RADIUS,
        },
        events::StageSpawnTrigger,
    },
    systems::camera::CameraPos,
    Layer,
};

pub fn read_stage_spawn_trigger(
    mut commands: Commands,
    mut event_reader: EventReader<StageSpawnTrigger>,
    mut assets_sprite: PxAssets<PxSprite>,
    camera_query: Query<&PxSubPosition, With<CameraPos>>,
) {
    let camera_pos = camera_query.get_single().unwrap();

    for event in event_reader.iter() {
        match &event.spawn {
            StageSpawn::Destructible(DestructibleSpawn {
                destructible_type, ..
            }) => match destructible_type {
                DestructibleType::Lamp => {}
                DestructibleType::Trashcan => {}
            },
            StageSpawn::Enemy(spawn) => spawn_enemy(&mut commands, &camera_pos, spawn),
            StageSpawn::Object(spawn) => spawn_object(&mut commands, &mut assets_sprite, spawn),
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

pub fn spawn_object(
    commands: &mut Commands,
    assets_sprite: &mut PxAssets<PxSprite>,
    spawn: &ObjectSpawn,
) {
    let (sprite_path, layer) = match spawn.object_type {
        ObjectType::BenchBig => ("sprites/objects/bench_big.png", Layer::Middle(1)),
        ObjectType::BenchSmall => ("sprites/objects/bench_small.png", Layer::Middle(1)),
        ObjectType::Fibertree => ("sprites/objects/fiber_tree.png", Layer::Middle(3)),
    };

    let sprite = assets_sprite.load(sprite_path);

    commands.spawn((
        Name::new(format!("Object {:?}", spawn.object_type)),
        Object {},
        PxSpriteBundle::<Layer> {
            sprite,
            anchor: PxAnchor::BottomCenter,
            layer,
            ..default()
        },
        PxSubPosition::from(spawn.coordinates.clone()),
    ));
}
