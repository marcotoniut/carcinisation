use bevy::prelude::*;
use seldom_pixel::{
    prelude::{PxAnchor, PxAssets, PxSubPosition},
    sprite::{PxSprite, PxSpriteBundle},
};

use crate::{
    stage::{
        components::{Collision, Destructible, Health, Hittable, Object, SpawnDrop},
        data::{
            DestructibleSpawn, DestructibleType, EnemySpawn, EnemyType, ObjectSpawn, ObjectType,
            PickupSpawn, PickupType, StageSpawn,
        },
        enemy::components::{
            Enemy, EnemyBehaviors, EnemyMosquito, EnemyMosquitoAttacking, EnemyTardigrade,
            EnemyTardigradeAttacking, ENEMY_MOSQUITO_BASE_HEALTH, ENEMY_MOSQUITO_RADIUS,
            ENEMY_TARDIGRADE_BASE_HEALTH, ENEMY_TARDIGRADE_RADIUS,
        },
        events::StageSpawnTrigger,
        pickup::components::HealthRecovery,
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
            StageSpawn::Destructible(spawn) => {
                spawn_destructible(&mut commands, &mut assets_sprite, spawn);
            }
            StageSpawn::Enemy(spawn) => {
                spawn_enemy(&mut commands, camera_pos.0, spawn);
            }
            StageSpawn::Object(spawn) => {
                spawn_object(&mut commands, &mut assets_sprite, spawn);
            }
            StageSpawn::Pickup(spawn) => {
                spawn_pickup(&mut commands, &mut assets_sprite, camera_pos.0, spawn);
            }
        }
    }
}

pub fn spawn_pickup(
    commands: &mut Commands,
    assets_sprite: &mut PxAssets<PxSprite>,
    offset: Vec2,
    spawn: &PickupSpawn,
) -> Entity {
    info!("Spawning Pickup {:?}", spawn.pickup_type);
    // TODO depth
    let depth = 1;
    let PickupSpawn {
        pickup_type,
        coordinates,
        elapsed,
    } = spawn;
    let position = PxSubPosition::from(offset + coordinates.clone());
    match pickup_type {
        PickupType::BigHealthpack => {
            let sprite = assets_sprite.load("sprites/pickups/health_2.png");
            commands
                .spawn((
                    Name::new(format!(
                        "Destructible BigHealthpack {:?}",
                        spawn.pickup_type
                    )),
                    Hittable {},
                    PxSpriteBundle::<Layer> {
                        sprite,
                        anchor: PxAnchor::BottomCenter,
                        layer: Layer::Middle(2),
                        ..default()
                    },
                    position,
                    Health(1),
                    Collision::Box(Vec2::new(12., 8.)),
                    HealthRecovery(100),
                ))
                .id()
        }
        PickupType::SmallHealthpack => {
            let sprite = assets_sprite.load("sprites/pickups/health_1.png");

            commands
                .spawn((
                    Name::new(format!(
                        "Destructible SmallHealthpack {:?}",
                        spawn.pickup_type
                    )),
                    Hittable {},
                    PxSpriteBundle::<Layer> {
                        sprite,
                        anchor: PxAnchor::BottomCenter,
                        layer: Layer::Middle(2),
                        ..default()
                    },
                    position,
                    Health(1),
                    Collision::Box(Vec2::new(7., 5.)),
                    HealthRecovery(30),
                ))
                .id()
        }
    }
}

pub fn spawn_enemy(commands: &mut Commands, offset: Vec2, enemy_spawn: &EnemySpawn) -> Entity {
    info!("Spawning Enemy {:?}", enemy_spawn.enemy_type);
    let EnemySpawn {
        enemy_type,
        coordinates,
        base_speed,
        steps,
        contains,
        ..
    } = enemy_spawn;
    let position = offset + *coordinates;
    let behaviors = EnemyBehaviors::new(steps.clone());
    match enemy_type {
        EnemyType::Mosquito => {
            let entity = commands
                .spawn((
                    Name::new("EnemyMosquito"),
                    Enemy {},
                    behaviors,
                    EnemyMosquito {
                        base_speed: *base_speed,
                        steps: steps.clone(),
                    },
                    EnemyMosquitoAttacking { ..default() },
                    Hittable {},
                    PxSubPosition::from(position),
                    Collision::Circle(ENEMY_MOSQUITO_RADIUS),
                    Health(ENEMY_MOSQUITO_BASE_HEALTH),
                ))
                .id();

            if let Some(contains) = contains {
                commands.entity(entity).insert(SpawnDrop {
                    contains: *contains.clone(),
                    entity,
                });
            }
            entity
        }
        EnemyType::Kyle => commands
            .spawn((Name::new("EnemyKyle"), Enemy {}, behaviors))
            .id(),
        EnemyType::Marauder => commands
            .spawn((Name::new("EnemyMarauder"), Enemy {}, behaviors))
            .id(),
        EnemyType::Spidey => commands
            .spawn((Name::new("EnemySpidey"), Enemy {}, behaviors))
            .id(),
        EnemyType::Spidomonsta => commands
            .spawn((Name::new("EnemySpidomonsta"), Enemy {}))
            .id(),
        EnemyType::Tardigrade => commands
            .spawn((
                Name::new("EnemyTardigrade"),
                Enemy {},
                behaviors,
                EnemyTardigrade {
                    steps: steps.clone(),
                },
                EnemyTardigradeAttacking { ..default() },
                Hittable {},
                PxSubPosition::from(position),
                Collision::Circle(ENEMY_TARDIGRADE_RADIUS),
                Health(ENEMY_TARDIGRADE_BASE_HEALTH),
            ))
            .id(),
    }
}

pub fn spawn_destructible(
    commands: &mut Commands,
    assets_sprite: &mut PxAssets<PxSprite>,
    spawn: &DestructibleSpawn,
) -> Entity {
    info!("Spawning Destructible {:?}", spawn.destructible_type);

    let (sprite_path, layer) = match spawn.destructible_type {
        DestructibleType::Lamp => ("sprites/objects/lamp.png", Layer::Middle(1)),
        DestructibleType::Trashcan => ("sprites/objects/trashcan.png", Layer::Middle(1)),
        DestructibleType::Crystal => todo!(),
        DestructibleType::Mushroom => todo!(),
    };
    let sprite = assets_sprite.load(sprite_path);
    commands
        .spawn((
            Name::new(format!("Destructible {:?}", spawn.destructible_type)),
            Destructible {},
            Hittable {},
            PxSpriteBundle::<Layer> {
                sprite,
                anchor: PxAnchor::BottomCenter,
                layer,
                ..default()
            },
            PxSubPosition::from(spawn.coordinates.clone()),
        ))
        .id()
}

pub fn spawn_object(
    commands: &mut Commands,
    assets_sprite: &mut PxAssets<PxSprite>,
    spawn: &ObjectSpawn,
) -> Entity {
    info!("Spawning Object {:?}", spawn.object_type);

    let (sprite_path, layer) = match spawn.object_type {
        ObjectType::BenchBig => ("sprites/objects/bench_big.png", Layer::Middle(1)),
        ObjectType::BenchSmall => ("sprites/objects/bench_small.png", Layer::Middle(1)),
        ObjectType::Fibertree => ("sprites/objects/fiber_tree.png", Layer::Middle(3)),
    };
    let sprite = assets_sprite.load(sprite_path);
    commands
        .spawn((
            Name::new(format!("Object {:?}", spawn.object_type)),
            Object {},
            PxSpriteBundle::<Layer> {
                sprite,
                anchor: PxAnchor::BottomCenter,
                layer,
                ..default()
            },
            PxSubPosition::from(spawn.coordinates.clone()),
        ))
        .id()
}
