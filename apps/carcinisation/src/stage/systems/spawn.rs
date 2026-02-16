//! Entity spawning systems for enemies, pickups, destructibles, and objects.

use crate::pixel::{PxAssets, PxSpriteBundle};
use crate::stage::{
    components::{
        SpawnDrop, StageEntity,
        interactive::{Collider, ColliderData, Dead},
        placement::Depth,
    },
    data::ContainerSpawn,
    destructible::{
        components::{DestructibleState, make_animation_bundle},
        data::destructibles::DESTRUCTIBLE_ANIMATIONS,
    },
    enemy::{
        entity::EnemyType,
        mosquito::entity::{ENEMY_MOSQUITO_RADIUS, MosquitoBundle},
        tardigrade::entity::{ENEMY_TARDIGRADE_RADIUS, TardigradeBundle},
    },
    player::{attacks::AttackHitTracker, components::PlayerAttack},
    resources::{StageStepSpawner, StageTimeDomain},
};
use crate::{
    layer::Layer,
    stage::{
        components::{
            interactive::{Flickerer, Health, Hittable, Object},
            placement::Speed,
        },
        data::{EnemySpawn, ObjectSpawn, ObjectType, PickupSpawn, PickupType, StageSpawn},
        destructible::{components::Destructible, data::DestructibleSpawn},
        enemy::components::{Enemy, behavior::EnemyBehaviors},
        messages::StageSpawnEvent,
        pickup::components::HealthRecovery,
    },
    systems::camera::CameraPos,
};
use assert_assets_path::assert_assets_path;
use bevy::prelude::*;
use seldom_pixel::prelude::{PxAnchor, PxSprite, PxSubPosition};

/// @system Drains the step spawner queue, triggering spawns whose elapsed time has come.
pub fn check_step_spawn(
    mut commands: Commands,
    mut stage_step_spawner_query: Query<&mut StageStepSpawner>,
    stage_time: Res<Time<StageTimeDomain>>,
) {
    for mut stage_step_spawner in &mut stage_step_spawner_query.iter_mut() {
        let mut elapsed = stage_step_spawner.elapsed + stage_time.delta();

        stage_step_spawner.spawns.retain_mut(|spawn| {
            let spawn_elapsed = spawn.get_elapsed();
            if spawn_elapsed <= elapsed {
                elapsed -= spawn_elapsed;
                commands.trigger(StageSpawnEvent {
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

/// @trigger Spawns an entity (enemy, pickup, destructible, or object) from a stage spawn event.
pub fn on_stage_spawn(
    trigger: On<StageSpawnEvent>,
    mut commands: Commands,
    mut assets_sprite: PxAssets<PxSprite>,
    camera_query: Query<&PxSubPosition, With<CameraPos>>,
) {
    match &trigger.event().spawn {
        StageSpawn::Destructible(x) => {
            spawn_destructible(&mut commands, &mut assets_sprite, x);
        }
        StageSpawn::Enemy(x) => {
            let camera_pos = camera_query.single().unwrap();
            spawn_enemy(&mut commands, camera_pos.0, x);
        }
        StageSpawn::Object(x) => {
            spawn_object(&mut commands, &mut assets_sprite, x);
        }
        StageSpawn::Pickup(x) => {
            let camera_pos = camera_query.single().unwrap();
            spawn_pickup(&mut commands, &mut assets_sprite, camera_pos.0, x);
        }
    }
}

pub fn spawn_pickup(
    commands: &mut Commands,
    assets_sprite: &mut PxAssets<PxSprite>,
    offset: Vec2,
    spawn: &PickupSpawn,
) -> Entity {
    // TODO depth
    let PickupSpawn {
        pickup_type,
        coordinates,
        ..
    } = spawn;
    let position = PxSubPosition::from(offset + *coordinates);
    match pickup_type {
        PickupType::BigHealthpack => {
            let sprite = assets_sprite.load(assert_assets_path!(
                "sprites/pickups/health_6.px_sprite.png"
            ));
            commands
                .spawn((
                    spawn.get_name(),
                    Hittable,
                    PxSpriteBundle::<Layer> {
                        sprite: sprite.into(),
                        anchor: PxAnchor::Center,
                        layer: spawn.depth.to_layer(),
                        ..default()
                    },
                    position,
                    spawn.depth,
                    Health(1),
                    ColliderData::from_one(Collider::new_box(Vec2::new(12., 8.))),
                    HealthRecovery(100),
                ))
                .id()
        }
        PickupType::SmallHealthpack => {
            let sprite = assets_sprite.load(assert_assets_path!(
                "sprites/pickups/health_4.px_sprite.png"
            ));
            commands
                .spawn((
                    spawn.get_name(),
                    Hittable,
                    PxSpriteBundle::<Layer> {
                        sprite: sprite.into(),
                        anchor: PxAnchor::BottomCenter,
                        layer: spawn.depth.to_layer(),
                        ..default()
                    },
                    position,
                    spawn.depth,
                    Health(1),
                    ColliderData::from_one(Collider::new_box(Vec2::new(7., 5.))),
                    HealthRecovery(30),
                ))
                .id()
        }
    }
}

pub fn spawn_enemy(commands: &mut Commands, offset: Vec2, spawn: &EnemySpawn) -> Entity {
    let EnemySpawn {
        enemy_type,
        coordinates,
        speed,
        steps,
        contains,
        depth,
        ..
    } = spawn;
    let name = spawn.enemy_type.get_name();
    let position = offset + *coordinates;
    let behaviors = EnemyBehaviors::new(steps.clone());
    match enemy_type {
        EnemyType::Mosquito => {
            let collider: Collider =
                Collider::new_circle(ENEMY_MOSQUITO_RADIUS).with_offset(Vec2::new(0., 2.));
            let critical_collider = collider.new_scaled(0.4).with_defense(0.4);

            let entity = commands
                .spawn(MosquitoBundle {
                    depth: *depth,
                    speed: Speed(*speed),
                    behaviors,
                    position: PxSubPosition::from(position),
                    collider_data: ColliderData::from_many(vec![critical_collider, collider]),
                    default: default(),
                })
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
        EnemyType::Tardigrade => {
            let collider =
                Collider::new_circle(ENEMY_TARDIGRADE_RADIUS).with_offset(Vec2::new(-3., 2.));
            let critical_collider = collider.new_scaled(0.4).with_defense(0.2);

            commands
                .spawn(TardigradeBundle {
                    depth: *depth,
                    speed: Speed(*speed),
                    behaviors,
                    position: PxSubPosition::from(position),
                    collider_data: ColliderData::from_many(vec![critical_collider, collider]),
                    default: default(),
                })
                .id()
        }
    }
}

/// Spawns a destructible entity with its animation bundle.
// TODO move to Destructible mod?
pub fn spawn_destructible(
    commands: &mut Commands,
    assets_sprite: &mut PxAssets<PxSprite>,
    spawn: &DestructibleSpawn,
) -> Entity {
    let animations_map = &DESTRUCTIBLE_ANIMATIONS.get_animation_data(&spawn.destructible_type);
    let animation_bundle_o = make_animation_bundle(
        assets_sprite,
        animations_map,
        &DestructibleState::Base,
        &spawn.depth,
    );
    let animation_bundle = animation_bundle_o.unwrap();

    commands
        .spawn((
            Destructible,
            Flickerer,
            Hittable,
            spawn.get_name(),
            spawn.depth,
            Health(spawn.health),
            spawn.destructible_type,
            animation_bundle,
            PxSubPosition::from(spawn.coordinates),
            StageEntity,
        ))
        .id()
}

pub fn spawn_object(
    commands: &mut Commands,
    assets_sprite: &mut PxAssets<PxSprite>,
    spawn: &ObjectSpawn,
) -> Entity {
    let sprite = assets_sprite.load(match spawn.object_type {
        ObjectType::BenchBig => assert_assets_path!("sprites/objects/bench_big.px_sprite.png"),
        ObjectType::BenchSmall => assert_assets_path!("sprites/objects/bench_small.px_sprite.png"),
        ObjectType::Fibertree => assert_assets_path!("sprites/objects/fiber_tree.px_sprite.png"),
        ObjectType::RugparkSign => {
            assert_assets_path!("sprites/objects/rugpark_sign.px_sprite.png")
        }
    });
    commands
        .spawn((
            spawn.get_name(),
            Object,
            PxSpriteBundle::<Layer> {
                sprite: sprite.into(),
                anchor: PxAnchor::BottomCenter,
                layer: spawn.depth.to_layer(),
                ..default()
            },
            PxSubPosition::from(spawn.coordinates),
            StageEntity,
        ))
        .id()
}

/// @system Spawns contained items when a carrier entity dies.
pub fn check_dead_drop(
    mut commands: Commands,
    mut assets_sprite: PxAssets<PxSprite>,
    mut attack_query: Query<&mut AttackHitTracker, With<PlayerAttack>>,
    query: Query<(&SpawnDrop, &PxSubPosition, &Depth), Added<Dead>>,
) {
    for (spawn_drop, position, depth) in &mut query.iter() {
        let entity = match spawn_drop.contains.clone() {
            ContainerSpawn::Pickup(spawn) => spawn_pickup(
                &mut commands,
                &mut assets_sprite,
                Vec2::ZERO,
                &spawn.from_spawn(position.0, *depth),
            ),
            ContainerSpawn::Enemy(spawn) => spawn_enemy(
                &mut commands,
                Vec2::ZERO,
                &spawn.from_spawn(position.0, *depth),
            ),
        };

        for mut hit_tracker in &mut attack_query.iter_mut() {
            hit_tracker.inherit_hit(spawn_drop.entity, entity);
        }
    }
}
