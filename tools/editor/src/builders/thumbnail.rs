use carcinisation::stage::{
    data::{ObjectType, PickupType},
    destructible::components::DestructibleType,
    enemy::entity::EnemyType,
};

pub fn get_enemy_thumbnail(enemy_type: EnemyType) -> (String, u32, u32) {
    match enemy_type {
        EnemyType::Mosquito => ("sprites/enemies/mosquito_idle_6.png".into(), 0, 0),
        EnemyType::Spidey => ("sprites/enemies/spider_idle_7.png".into(), 0, 0),
        EnemyType::Tardigrade => ("sprites/enemies/tardigrade_idle_6.png".into(), 0, 0),
        EnemyType::Marauder => ("sprites/enemies/tardigrade_idle_6.png".into(), 0, 0),
        EnemyType::Spidomonsta => ("sprites/enemies/tardigrade_idle_6.png".into(), 0, 0),
        EnemyType::Kyle => ("sprites/enemies/tardigrade_idle_6.png".into(), 0, 0),
    }
}

pub fn get_destructible_thumbnail(destructible_type: DestructibleType) -> (String, u32, u32) {
    match destructible_type {
        DestructibleType::Crystal => ("sprites/objects/crystal_base_5.png".into(), 0, 0),
        DestructibleType::Lamp => ("sprites/objects/lamp_base_3.png".into(), 0, 0),
        DestructibleType::Mushroom => ("sprites/objects/mushroom_base_4.png".into(), 0, 0),
        DestructibleType::Trashcan => ("sprites/objects/trashcan_base_6.png".into(), 0, 0),
    }
}

pub fn get_object_thumbnail(object_type: ObjectType) -> (String, u32, u32) {
    match object_type {
        ObjectType::BenchBig => ("sprites/objects/bench_big.png".into(), 0, 0),
        ObjectType::BenchSmall => ("sprites/objects/bench_small.png".into(), 0, 0),
        ObjectType::Fibertree => ("sprites/objects/fiber_tree.png".into(), 0, 0),
        ObjectType::RugparkSign => ("sprites/objects/rugpark_sign.png".into(), 0, 0),
    }
}

pub fn get_pickup_thumbnail(pickup_type: PickupType) -> (String, u32, u32) {
    match pickup_type {
        PickupType::BigHealthpack => ("sprites/pickups/health_6.png".into(), 0, 0),
        PickupType::SmallHealthpack => ("sprites/pickups/health_6.png".into(), 0, 0),
    }
}
