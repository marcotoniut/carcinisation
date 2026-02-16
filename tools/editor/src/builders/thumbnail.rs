use bevy::math::{Rect, URect};
use carcinisation::stage::{
    components::placement::Depth,
    data::{ObjectType, PickupType},
    destructible::components::DestructibleType,
    enemy::entity::EnemyType,
};

pub fn get_enemy_thumbnail(enemy_type: EnemyType, depth: Depth) -> (String, Option<Rect>) {
    match enemy_type {
        EnemyType::Mosquito => {
            let loc = "sprites/enemies/mosquito_idle_";
            let ext = ".px_sprite.png";
            match depth {
                Depth::Three => (
                    format!("{loc}3{ext}"),
                    URect::new(0, 0, 49, 49).as_rect().into(),
                ),
                Depth::Four => (
                    format!("{loc}4{ext}"),
                    URect::new(0, 0, 35, 35).as_rect().into(),
                ),
                Depth::Five => (
                    format!("{loc}5{ext}"),
                    URect::new(0, 0, 23, 23).as_rect().into(),
                ),
                Depth::Six => (
                    format!("{loc}6{ext}"),
                    URect::new(0, 0, 15, 15).as_rect().into(),
                ),
                Depth::Seven => (
                    format!("{loc}7{ext}"),
                    URect::new(0, 0, 9, 9).as_rect().into(),
                ),
                Depth::Eight => (
                    format!("{loc}8{ext}"),
                    URect::new(0, 0, 5, 5).as_rect().into(),
                ),
                _ => panic!("{} Invalid depth {}", loc, depth.to_i8()),
            }
        }
        EnemyType::Spidey => {
            let loc = "sprites/enemies/spider_idle_";
            let ext = ".px_sprite.png";
            match depth {
                Depth::Two => (
                    format!("{loc}2{ext}"),
                    URect::new(0, 0, 49, 49).as_rect().into(),
                ),
                Depth::Seven => (
                    format!("{loc}7{ext}"),
                    URect::new(0, 0, 35, 35).as_rect().into(),
                ),
                _ => panic!("{} Invalid depth {}", loc, depth.to_i8()),
            }
        }
        EnemyType::Tardigrade => {
            let loc = "sprites/enemies/tardigrade_idle_";
            let ext = ".px_sprite.png";
            match depth {
                Depth::Six => (
                    format!("{loc}6{ext}"),
                    URect::new(0, 0, 63, 63).as_rect().into(),
                ),
                Depth::Seven => (
                    format!("{loc}7{ext}"),
                    URect::new(0, 0, 42, 42).as_rect().into(),
                ),
                Depth::Eight => (
                    format!("{loc}8{ext}"),
                    URect::new(0, 0, 23, 23).as_rect().into(),
                ),
                _ => panic!("{} Invalid depth {}", loc, depth.to_i8()),
            }
        }
        EnemyType::Marauder => {
            let loc = "sprites/enemies/marauder_idle_";
            let ext = ".px_sprite.png";
            panic!("{}{} Invalid depth {}", loc, ext, depth.to_i8());
        }
        EnemyType::Spidomonsta => {
            let loc = "sprites/enemies/spidomonsta_idle_";
            let ext = ".px_sprite.png";
            panic!("{}{} Invalid depth {}", loc, ext, depth.to_i8());
        }
        EnemyType::Kyle => {
            let loc = "sprites/enemies/kyle_idle_";
            let ext = ".px_sprite.png";
            panic!("{}{} Invalid depth {}", loc, ext, depth.to_i8());
        }
    }
}

pub fn get_destructible_thumbnail(
    destructible_type: DestructibleType,
    _depth: Depth,
) -> (String, Option<Rect>) {
    match destructible_type {
        DestructibleType::Crystal => ("sprites/objects/crystal_base_5.px_sprite.png".into(), None),
        DestructibleType::Lamp => ("sprites/objects/lamp_base_3.px_sprite.png".into(), None),
        DestructibleType::Mushroom => {
            ("sprites/objects/mushroom_base_4.px_sprite.png".into(), None)
        }
        DestructibleType::Trashcan => {
            ("sprites/objects/trashcan_base_6.px_sprite.png".into(), None)
        }
    }
}

pub fn get_object_thumbnail(object_type: ObjectType, _depth: Depth) -> (String, Option<Rect>) {
    match object_type {
        ObjectType::BenchBig => ("sprites/objects/bench_big.px_sprite.png".into(), None),
        ObjectType::BenchSmall => ("sprites/objects/bench_small.px_sprite.png".into(), None),
        ObjectType::Fibertree => ("sprites/objects/fiber_tree.px_sprite.png".into(), None),
        ObjectType::RugparkSign => ("sprites/objects/rugpark_sign.px_sprite.png".into(), None),
    }
}

pub fn get_pickup_thumbnail(pickup_type: PickupType, _depth: Depth) -> (String, Option<Rect>) {
    match pickup_type {
        PickupType::BigHealthpack => ("sprites/pickups/health_6.px_sprite.png".into(), None),
        PickupType::SmallHealthpack => ("sprites/pickups/health_4.px_sprite.png".into(), None),
    }
}
