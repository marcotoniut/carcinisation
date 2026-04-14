use bevy::prelude::*;
use carcinisation::stage::{
    components::placement::Depth,
    data::{EnemySpawn, ObjectSpawn, ObjectType, PickupSpawn, PickupType, StageSpawn},
    destructible::{components::DestructibleType, data::DestructibleSpawn},
    enemy::entity::EnemyType,
};

use crate::builders::thumbnail::get_enemy_thumbnail;
use carcinisation::stage::enemy::data::{mosquiton, spidey};

/// Depths available in the editor spawn palette (excludes 0 = player layer).
pub const EDITOR_DEPTHS: &[Depth] = &[
    Depth::One,
    Depth::Two,
    Depth::Three,
    Depth::Four,
    Depth::Five,
    Depth::Six,
    Depth::Seven,
    Depth::Eight,
    Depth::Nine,
];

/// Describes which spawn type to create when the user clicks.
#[derive(Clone, Debug)]
pub enum SpawnTemplate {
    Object(ObjectType),
    Destructible(DestructibleType),
    Pickup(PickupType),
    Enemy(EnemyType),
}

impl SpawnTemplate {
    /// Human-readable label for the palette.
    pub fn label(&self) -> &'static str {
        match self {
            SpawnTemplate::Object(ObjectType::BenchBig) => "Bench (Big)",
            SpawnTemplate::Object(ObjectType::BenchSmall) => "Bench (Small)",
            SpawnTemplate::Object(ObjectType::Fibertree) => "Fiber Tree",
            SpawnTemplate::Object(ObjectType::RugparkSign) => "Rugpark Sign",
            SpawnTemplate::Destructible(DestructibleType::Lamp) => "Lamp",
            SpawnTemplate::Destructible(DestructibleType::Trashcan) => "Trashcan",
            SpawnTemplate::Destructible(DestructibleType::Crystal) => "Crystal",
            SpawnTemplate::Destructible(DestructibleType::Mushroom) => "Mushroom",
            SpawnTemplate::Pickup(PickupType::BigHealthpack) => "Health (Big)",
            SpawnTemplate::Pickup(PickupType::SmallHealthpack) => "Health (Small)",
            SpawnTemplate::Enemy(EnemyType::Mosquito) => "Mosquito",
            SpawnTemplate::Enemy(EnemyType::Mosquiton) => "Mosquiton",
            SpawnTemplate::Enemy(EnemyType::Tardigrade) => "Tardigrade",
            SpawnTemplate::Enemy(EnemyType::Spidey) => "Spidey",
            SpawnTemplate::Enemy(EnemyType::Marauder) => "Marauder",
            SpawnTemplate::Enemy(EnemyType::Spidomonsta) => "Spidomonsta",
            SpawnTemplate::Enemy(EnemyType::Kyle) => "Kyle",
        }
    }

    /// Instantiate a `StageSpawn` at the given coordinates with the given depth.
    pub fn instantiate(&self, coordinates: Vec2, depth: Depth) -> StageSpawn {
        match self {
            SpawnTemplate::Object(ObjectType::BenchBig) => StageSpawn::Object(
                ObjectSpawn::bench_big_base(coordinates.x, coordinates.y).with_depth(depth),
            ),
            SpawnTemplate::Object(ObjectType::BenchSmall) => StageSpawn::Object(
                ObjectSpawn::bench_small_base(coordinates.x, coordinates.y).with_depth(depth),
            ),
            SpawnTemplate::Object(ObjectType::Fibertree) => StageSpawn::Object(
                ObjectSpawn::fibertree_base(coordinates.x, coordinates.y).with_depth(depth),
            ),
            SpawnTemplate::Object(ObjectType::RugparkSign) => StageSpawn::Object(
                ObjectSpawn::rugpark_sign_base(coordinates.x, coordinates.y).with_depth(depth),
            ),
            SpawnTemplate::Destructible(DestructibleType::Lamp) => {
                StageSpawn::Destructible(DestructibleSpawn {
                    destructible_type: DestructibleType::Lamp,
                    coordinates,
                    depth,
                    health: 60,
                    contains: None,
                    authored_depths: None,
                })
            }
            SpawnTemplate::Destructible(DestructibleType::Trashcan) => {
                StageSpawn::Destructible(DestructibleSpawn {
                    destructible_type: DestructibleType::Trashcan,
                    coordinates,
                    depth,
                    health: 100,
                    contains: None,
                    authored_depths: None,
                })
            }
            SpawnTemplate::Destructible(DestructibleType::Crystal) => {
                StageSpawn::Destructible(DestructibleSpawn {
                    destructible_type: DestructibleType::Crystal,
                    coordinates,
                    depth,
                    health: 300,
                    contains: None,
                    authored_depths: None,
                })
            }
            SpawnTemplate::Destructible(DestructibleType::Mushroom) => {
                StageSpawn::Destructible(DestructibleSpawn {
                    destructible_type: DestructibleType::Mushroom,
                    coordinates,
                    depth,
                    health: 120,
                    contains: None,
                    authored_depths: None,
                })
            }
            SpawnTemplate::Pickup(PickupType::BigHealthpack) => StageSpawn::Pickup(
                PickupSpawn::big_healthpack_base()
                    .with_coordinates(coordinates)
                    .with_depth(depth),
            ),
            SpawnTemplate::Pickup(PickupType::SmallHealthpack) => StageSpawn::Pickup(
                PickupSpawn::small_healthpack_base()
                    .with_coordinates(coordinates)
                    .with_depth(depth),
            ),
            SpawnTemplate::Enemy(enemy_type) => {
                let base = match enemy_type {
                    EnemyType::Mosquito => EnemySpawn::mosquito_base(),
                    EnemyType::Mosquiton => EnemySpawn::mosquiton_base(),
                    EnemyType::Tardigrade => EnemySpawn::tardigrade_base(),
                    EnemyType::Spidey => EnemySpawn::spidey_base(1.0, coordinates),
                    _ => EnemySpawn::mosquito_base().with_enemy_type(*enemy_type),
                };
                StageSpawn::Enemy(base.with_coordinates(coordinates).with_depth(depth))
            }
        }
    }

    /// Whether this template has a valid sprite at the given depth.
    pub fn has_sprite_at_depth(&self, depth: Depth) -> bool {
        match self {
            SpawnTemplate::Object(object_type) => match object_type {
                ObjectType::BenchBig | ObjectType::BenchSmall => {
                    matches!(depth, Depth::Six | Depth::Seven | Depth::Eight)
                }
                ObjectType::Fibertree => matches!(depth, Depth::Two | Depth::Three),
                ObjectType::RugparkSign => matches!(depth, Depth::Three | Depth::Four),
            },
            SpawnTemplate::Destructible(dt) => match dt {
                DestructibleType::Lamp => depth == Depth::Three,
                DestructibleType::Trashcan => matches!(depth, Depth::Four | Depth::Six),
                DestructibleType::Crystal => depth == Depth::Five,
                DestructibleType::Mushroom => depth == Depth::Four,
            },
            SpawnTemplate::Pickup(_) => {
                matches!(depth, Depth::Four | Depth::Five | Depth::Six)
            }
            SpawnTemplate::Enemy(enemy_type) => {
                // Check composed atlas or spritesheet.
                let base = crate::constants::assets_root().join(format!(
                    "sprites/enemies/{}_{}",
                    enemy_type.sprite_base_name(),
                    depth.to_i8()
                ));
                base.join("atlas.json").exists()
                    || get_enemy_thumbnail(*enemy_type, depth).is_some()
            }
        }
    }

    /// Returns the default depth for this template.
    pub fn default_depth(&self) -> Depth {
        match self {
            SpawnTemplate::Object(ObjectType::BenchBig | ObjectType::BenchSmall) => Depth::Eight,
            SpawnTemplate::Object(ObjectType::Fibertree) => Depth::Two,
            SpawnTemplate::Object(ObjectType::RugparkSign) => Depth::Three,
            SpawnTemplate::Destructible(DestructibleType::Lamp) => Depth::Three,
            SpawnTemplate::Destructible(DestructibleType::Trashcan) => Depth::Six,
            SpawnTemplate::Destructible(DestructibleType::Crystal) => Depth::Five,
            SpawnTemplate::Destructible(DestructibleType::Mushroom) => Depth::Four,
            SpawnTemplate::Pickup(_) => Depth::Six,
            SpawnTemplate::Enemy(EnemyType::Mosquito) => Depth::Three,
            SpawnTemplate::Enemy(EnemyType::Mosquiton) => Depth::Three,
            SpawnTemplate::Enemy(EnemyType::Tardigrade) => Depth::Six,
            SpawnTemplate::Enemy(EnemyType::Spidey) => Depth::Three,
            SpawnTemplate::Enemy(_) => Depth::Three,
        }
    }

    /// All available templates, grouped for the palette UI.
    pub fn all_objects() -> Vec<SpawnTemplate> {
        vec![
            SpawnTemplate::Object(ObjectType::BenchBig),
            SpawnTemplate::Object(ObjectType::BenchSmall),
            SpawnTemplate::Object(ObjectType::Fibertree),
            SpawnTemplate::Object(ObjectType::RugparkSign),
        ]
    }

    pub fn all_destructibles() -> Vec<SpawnTemplate> {
        vec![
            SpawnTemplate::Destructible(DestructibleType::Lamp),
            SpawnTemplate::Destructible(DestructibleType::Trashcan),
            SpawnTemplate::Destructible(DestructibleType::Crystal),
            SpawnTemplate::Destructible(DestructibleType::Mushroom),
        ]
    }

    pub fn all_pickups() -> Vec<SpawnTemplate> {
        vec![
            SpawnTemplate::Pickup(PickupType::BigHealthpack),
            SpawnTemplate::Pickup(PickupType::SmallHealthpack),
        ]
    }

    pub fn all_enemies() -> Vec<SpawnTemplate> {
        vec![
            SpawnTemplate::Enemy(EnemyType::Mosquito),
            SpawnTemplate::Enemy(EnemyType::Mosquiton),
            SpawnTemplate::Enemy(EnemyType::Tardigrade),
            SpawnTemplate::Enemy(EnemyType::Spidey),
            SpawnTemplate::Enemy(EnemyType::Marauder),
            SpawnTemplate::Enemy(EnemyType::Spidomonsta),
            SpawnTemplate::Enemy(EnemyType::Kyle),
        ]
    }

    /// Available animation tags for composed enemies. `None` for non-composed types.
    pub fn available_animation_tags(&self) -> Option<&'static [&'static str]> {
        match self {
            SpawnTemplate::Enemy(EnemyType::Mosquiton) => Some(mosquiton::GALLERY_TAGS),
            SpawnTemplate::Enemy(EnemyType::Spidey) => Some(spidey::GALLERY_TAGS),
            _ => None,
        }
    }

    /// Default animation tag for preview. `None` for non-composed types.
    pub fn default_animation_tag(&self) -> Option<&'static str> {
        match self {
            SpawnTemplate::Enemy(EnemyType::Mosquiton) => Some(mosquiton::TAG_IDLE_FLY),
            SpawnTemplate::Enemy(EnemyType::Spidey) => Some(spidey::TAG_IDLE),
            _ => None,
        }
    }
}

/// Active placement mode state.
#[derive(Resource, Default, Debug)]
pub struct PlacementMode {
    pub active: Option<PlacementState>,
}

#[derive(Clone, Debug)]
pub struct PlacementState {
    pub template: SpawnTemplate,
    pub depth: Depth,
    /// Selected animation tag for composed-enemy preview. `None` = default idle.
    pub animation_tag: Option<String>,
}
