use crate::stage::{
    components::{
        interactive::{CollisionData, Dead},
        placement::Depth,
        SpawnDrop, StageEntity,
    },
    data::ContainerSpawn,
    destructible::{
        components::{make_animation_bundle, DestructibleState},
        data::destructibles::DESTRUCTIBLE_ANIMATIONS,
    },
    enemy::components::ENEMY_TARDIGRADE_RADIUS,
    player::components::{PlayerAttack, UnhittableList},
    resources::{StageStepSpawner, StageTime},
};
use crate::{
    stage::{
        components::{
            interactive::{Collision, Flickerer, Health, Hittable, Object},
            placement::Speed,
        },
        data::{
            EnemySpawn, EnemyType, ObjectSpawn, ObjectType, PickupSpawn, PickupType, StageSpawn,
        },
        destructible::{components::Destructible, data::DestructibleSpawn},
        enemy::components::{
            behavior::EnemyBehaviors, Enemy, EnemyMosquito, EnemyMosquitoAttacking,
            EnemyTardigrade, EnemyTardigradeAttacking, ENEMY_MOSQUITO_BASE_HEALTH,
            ENEMY_MOSQUITO_RADIUS, ENEMY_TARDIGRADE_BASE_HEALTH,
        },
        events::StageSpawnEvent,
        pickup::components::HealthRecovery,
    },
    systems::camera::CameraPos,
    Layer,
};
use bevy::prelude::*;
use seldom_pixel::{
    prelude::{PxAnchor, PxAssets, PxSubPosition},
    sprite::{PxSprite, PxSpriteBundle},
};

pub fn check_step_spawn(
    mut event_writer: EventWriter<StageSpawnEvent>,
    mut stage_step_spawner_query: Query<&mut StageStepSpawner>,
    stage_time: Res<StageTime>,
) {
    for mut stage_step_spawner in &mut stage_step_spawner_query.iter_mut() {
        let mut elapsed = stage_step_spawner.elapsed + stage_time.delta;

        stage_step_spawner.spawns.retain_mut(|spawn| {
            let spawn_elapsed = spawn.get_elapsed();
            if spawn_elapsed <= elapsed {
                elapsed -= spawn_elapsed;
                event_writer.send(StageSpawnEvent {
                    spawn: spawn.clone(),
                });
                false
            } else {
                true
            }
        });

        stage_step_spawner.elapsed = elapsed;
    }
}

pub fn read_stage_spawn_trigger(
    mut commands: Commands,
    mut event_reader: EventReader<StageSpawnEvent>,
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
    let name = Name::new(format!("Pickup {:?}", spawn.pickup_type));
    info!("Spawning {:?}", name.as_str());
    // TODO depth
    let PickupSpawn {
        pickup_type,
        coordinates,
        ..
    } = spawn;
    let position = PxSubPosition::from(offset + coordinates.clone());
    match pickup_type {
        PickupType::BigHealthpack => {
            let sprite = assets_sprite.load("sprites/pickups/health_2.png");
            commands
                .spawn((
                    name,
                    Hittable,
                    PxSpriteBundle::<Layer> {
                        sprite,
                        anchor: PxAnchor::Center,
                        layer: Layer::Middle(2),
                        ..Default::default()
                    },
                    position,
                    // TODO should this be the spawn depth or the spawner's?
                    Depth(spawn.depth),
                    Health(1),
                    CollisionData::new(Collision::Box(Vec2::new(12., 8.))),
                    HealthRecovery(100),
                ))
                .id()
        }
        PickupType::SmallHealthpack => {
            let sprite = assets_sprite.load("sprites/pickups/health_1.png");

            commands
                .spawn((
                    name,
                    Hittable,
                    PxSpriteBundle::<Layer> {
                        sprite,
                        anchor: PxAnchor::BottomCenter,
                        layer: Layer::Middle(2),
                        ..Default::default()
                    },
                    position,
                    Depth(spawn.depth),
                    Health(1),
                    CollisionData::new(Collision::Box(Vec2::new(7., 5.))),
                    HealthRecovery(30),
                ))
                .id()
        }
    }
}

pub fn spawn_enemy(commands: &mut Commands, offset: Vec2, enemy_spawn: &EnemySpawn) -> Entity {
    let EnemySpawn {
        enemy_type,
        coordinates,
        speed,
        steps,
        contains,
        depth,
        ..
    } = enemy_spawn;
    let name = Name::new(format!("Enemy {:?}", enemy_type));
    info!("Spawning {:?}", name.as_str());
    let position = offset + *coordinates;
    let behaviors = EnemyBehaviors::new(steps.clone());
    match enemy_type {
        EnemyType::Mosquito => {
            let entity = commands
                .spawn((
                    name,
                    StageEntity,
                    Enemy,
                    behaviors,
                    Speed(*speed),
                    EnemyMosquito {
                        steps: steps.clone(),
                    },
                    Depth(*depth),
                    EnemyMosquitoAttacking {
                        ..Default::default()
                    },
                    Flickerer,
                    Hittable,
                    PxSubPosition::from(position),
                    CollisionData::new(Collision::Circle(ENEMY_MOSQUITO_RADIUS))
                        .with_offset(Vec2::new(0., 2.)),
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
        EnemyType::Kyle => commands.spawn((name, Enemy, behaviors)).id(),
        EnemyType::Marauder => commands.spawn((name, Enemy, behaviors)).id(),
        EnemyType::Spidey => commands.spawn((name, Enemy, behaviors)).id(),
        EnemyType::Spidomonsta => commands.spawn((name, Enemy, behaviors)).id(),
        EnemyType::Tardigrade => commands
            .spawn((
                name,
                behaviors,
                Depth(*depth),
                StageEntity,
                Enemy,
                EnemyTardigrade {
                    steps: steps.clone(),
                },
                EnemyTardigradeAttacking {
                    ..Default::default()
                },
                Flickerer,
                Hittable,
                // TODO speed needs to be assigned otherwise the check_no_behavior function cannot find the target enemy
                Speed(*speed),
                PxSubPosition::from(position),
                CollisionData::new(Collision::Circle(ENEMY_TARDIGRADE_RADIUS))
                    .with_offset(Vec2::new(-3., 2.)),
                Health(ENEMY_TARDIGRADE_BASE_HEALTH),
            ))
            .id(),
    }
}

/**
 * TODO move to Destructible mod?
 */
pub fn spawn_destructible(
    commands: &mut Commands,
    assets_sprite: &mut PxAssets<PxSprite>,
    spawn: &DestructibleSpawn,
) -> Entity {
    let name = Name::new(format!("Destructible {:?}", spawn.destructible_type));
    info!("Spawning {:?}", name.as_str());

    let animations_map = &DESTRUCTIBLE_ANIMATIONS.get_animation_data(&spawn.destructible_type);
    let animation_bundle_o = make_animation_bundle(
        assets_sprite,
        animations_map,
        &DestructibleState::Base,
        spawn.depth,
    );
    let animation_bundle = animation_bundle_o.unwrap();

    commands
        .spawn((
            Destructible,
            Flickerer,
            Hittable,
            Depth(spawn.depth),
            Health(spawn.health),
            spawn.destructible_type.clone(),
            animation_bundle,
            PxSubPosition::from(spawn.coordinates.clone()),
            StageEntity,
        ))
        .id()
}

pub fn spawn_object(
    commands: &mut Commands,
    assets_sprite: &mut PxAssets<PxSprite>,
    spawn: &ObjectSpawn,
) -> Entity {
    let name = Name::new(format!("Object {:?}", spawn.object_type));
    info!("Spawning {:?}", name.as_str());

    let (sprite_path, layer) = match spawn.object_type {
        ObjectType::BenchBig => ("sprites/objects/bench_big.png", Layer::Middle(1)),
        ObjectType::BenchSmall => ("sprites/objects/bench_small.png", Layer::Middle(1)),
        ObjectType::Fibertree => ("sprites/objects/fiber_tree.png", Layer::Middle(5)),
        ObjectType::Rugpark => ("sprites/objects/rugpark.png", Layer::Middle(3)),
    };
    let sprite = assets_sprite.load(sprite_path);
    commands
        .spawn((
            name,
            Object,
            PxSpriteBundle::<Layer> {
                sprite,
                anchor: PxAnchor::BottomCenter,
                layer,
                ..Default::default()
            },
            PxSubPosition::from(spawn.coordinates.clone()),
            StageEntity,
        ))
        .id()
}

pub fn check_dead_drop(
    mut commands: Commands,
    mut assets_sprite: PxAssets<PxSprite>,
    mut attack_query: Query<&mut UnhittableList, With<PlayerAttack>>,
    query: Query<(&SpawnDrop, &PxSubPosition), Added<Dead>>,
) {
    for (spawn_drop, position) in &mut query.iter() {
        let entity = match spawn_drop.contains.clone() {
            ContainerSpawn::Pickup(mut spawn) => {
                spawn.coordinates = position.0;
                spawn_pickup(&mut commands, &mut assets_sprite, Vec2::ZERO, &spawn)
            }
            ContainerSpawn::Enemy(mut spawn) => {
                spawn.coordinates = position.0;
                spawn_enemy(&mut commands, Vec2::ZERO, &spawn)
            }
        };

        for mut unhittable_list in &mut attack_query.iter_mut() {
            if unhittable_list.0.contains(&spawn_drop.entity) {
                unhittable_list.0.insert(entity);
            }
        }
    }
}
