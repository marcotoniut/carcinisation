use bevy::prelude::*;
use carcinisation_ors::stage::{
    components::placement::Depth,
    data::{EnemySpawn, ObjectSpawn, ObjectType, PickupSpawn, PickupType, StageSpawn},
    destructible::{components::DestructibleType, data::DestructibleSpawn},
    enemy::entity::EnemyType,
};

use crate::builders::thumbnail::get_enemy_thumbnail;
use carcinisation_ors::stage::enemy::data::{mosquiton, spidey};

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
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Object(ObjectType::BenchBig) => "Bench (Big)",
            Self::Object(ObjectType::BenchSmall) => "Bench (Small)",
            Self::Object(ObjectType::Fibertree) => "Fiber Tree",
            Self::Object(ObjectType::RugparkSign) => "Rugpark Sign",
            Self::Destructible(DestructibleType::Lamp) => "Lamp",
            Self::Destructible(DestructibleType::Trashcan) => "Trashcan",
            Self::Destructible(DestructibleType::Crystal) => "Crystal",
            Self::Destructible(DestructibleType::Mushroom) => "Mushroom",
            Self::Pickup(PickupType::SmallHealth) => "Health (Small)",
            Self::Pickup(PickupType::BigHealth) => "Health (Big)",
            Self::Pickup(PickupType::Flamethrower) => "Flamethrower",
            Self::Pickup(PickupType::Bullet) => "Bullet",
            Self::Pickup(PickupType::Piercing) => "Piercing",
            Self::Pickup(PickupType::Bomb) => "Bomb",
            Self::Enemy(EnemyType::Mosquito) => "Mosquito",
            Self::Enemy(EnemyType::Mosquiton) => "Mosquiton",
            Self::Enemy(EnemyType::Tardigrade) => "Tardigrade",
            Self::Enemy(EnemyType::Spidey) => "Spidey",
            Self::Enemy(EnemyType::Marauder) => "Marauder",
            Self::Enemy(EnemyType::Spidomonsta) => "Spidomonsta",
            Self::Enemy(EnemyType::Kyle) => "Kyle",
        }
    }

    /// Instantiate a `StageSpawn` at the given coordinates with the given depth.
    pub fn instantiate(&self, coordinates: Vec2, depth: Depth) -> StageSpawn {
        match self {
            Self::Object(ObjectType::BenchBig) => StageSpawn::Object(
                ObjectSpawn::bench_big_base(coordinates.x, coordinates.y).with_depth(depth),
            ),
            Self::Object(ObjectType::BenchSmall) => StageSpawn::Object(
                ObjectSpawn::bench_small_base(coordinates.x, coordinates.y).with_depth(depth),
            ),
            Self::Object(ObjectType::Fibertree) => StageSpawn::Object(
                ObjectSpawn::fibertree_base(coordinates.x, coordinates.y).with_depth(depth),
            ),
            Self::Object(ObjectType::RugparkSign) => StageSpawn::Object(
                ObjectSpawn::rugpark_sign_base(coordinates.x, coordinates.y).with_depth(depth),
            ),
            Self::Destructible(DestructibleType::Lamp) => {
                StageSpawn::Destructible(DestructibleSpawn {
                    destructible_type: DestructibleType::Lamp,
                    coordinates,
                    depth,
                    health: 60,
                    contains: None,
                    authored_depths: None,
                })
            }
            Self::Destructible(DestructibleType::Trashcan) => {
                StageSpawn::Destructible(DestructibleSpawn {
                    destructible_type: DestructibleType::Trashcan,
                    coordinates,
                    depth,
                    health: 100,
                    contains: None,
                    authored_depths: None,
                })
            }
            Self::Destructible(DestructibleType::Crystal) => {
                StageSpawn::Destructible(DestructibleSpawn {
                    destructible_type: DestructibleType::Crystal,
                    coordinates,
                    depth,
                    health: 300,
                    contains: None,
                    authored_depths: None,
                })
            }
            Self::Destructible(DestructibleType::Mushroom) => {
                StageSpawn::Destructible(DestructibleSpawn {
                    destructible_type: DestructibleType::Mushroom,
                    coordinates,
                    depth,
                    health: 120,
                    contains: None,
                    authored_depths: None,
                })
            }
            Self::Pickup(pickup_type) => StageSpawn::Pickup(
                PickupSpawn::base(*pickup_type)
                    .with_coordinates(coordinates)
                    .with_depth(depth),
            ),
            Self::Enemy(enemy_type) => {
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
            Self::Object(object_type) => match object_type {
                ObjectType::BenchBig | ObjectType::BenchSmall => {
                    matches!(depth, Depth::Six | Depth::Seven | Depth::Eight)
                }
                ObjectType::Fibertree => matches!(depth, Depth::Two | Depth::Three),
                ObjectType::RugparkSign => matches!(depth, Depth::Three | Depth::Four),
            },
            Self::Destructible(dt) => match dt {
                DestructibleType::Lamp => depth == Depth::Three,
                DestructibleType::Trashcan => matches!(depth, Depth::Four | Depth::Six),
                DestructibleType::Crystal => depth == Depth::Five,
                DestructibleType::Mushroom => depth == Depth::Four,
            },
            Self::Pickup(_) => {
                matches!(depth, Depth::Four | Depth::Five | Depth::Six)
            }
            Self::Enemy(enemy_type) => {
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
    pub const fn default_depth(&self) -> Depth {
        match self {
            Self::Object(ObjectType::Fibertree) => Depth::Two,
            Self::Destructible(DestructibleType::Mushroom) => Depth::Four,
            Self::Destructible(DestructibleType::Crystal) => Depth::Five,
            Self::Destructible(DestructibleType::Trashcan)
            | Self::Pickup(_)
            | Self::Enemy(EnemyType::Tardigrade) => Depth::Six,
            Self::Object(ObjectType::BenchBig | ObjectType::BenchSmall) => Depth::Eight,
            Self::Object(ObjectType::RugparkSign)
            | Self::Destructible(DestructibleType::Lamp)
            | Self::Enemy(_) => Depth::Three,
        }
    }

    /// All available templates, grouped for the palette UI.
    pub fn all_objects() -> Vec<Self> {
        vec![
            Self::Object(ObjectType::BenchBig),
            Self::Object(ObjectType::BenchSmall),
            Self::Object(ObjectType::Fibertree),
            Self::Object(ObjectType::RugparkSign),
        ]
    }

    pub fn all_destructibles() -> Vec<Self> {
        vec![
            Self::Destructible(DestructibleType::Lamp),
            Self::Destructible(DestructibleType::Trashcan),
            Self::Destructible(DestructibleType::Crystal),
            Self::Destructible(DestructibleType::Mushroom),
        ]
    }

    pub fn all_pickups() -> Vec<Self> {
        vec![
            Self::Pickup(PickupType::SmallHealth),
            Self::Pickup(PickupType::BigHealth),
            Self::Pickup(PickupType::Flamethrower),
            Self::Pickup(PickupType::Bullet),
            Self::Pickup(PickupType::Piercing),
            Self::Pickup(PickupType::Bomb),
        ]
    }

    pub fn all_enemies() -> Vec<Self> {
        vec![
            Self::Enemy(EnemyType::Mosquito),
            Self::Enemy(EnemyType::Mosquiton),
            Self::Enemy(EnemyType::Tardigrade),
            Self::Enemy(EnemyType::Spidey),
            Self::Enemy(EnemyType::Marauder),
            Self::Enemy(EnemyType::Spidomonsta),
            Self::Enemy(EnemyType::Kyle),
        ]
    }

    /// Available animation actions for composed enemies. `None` for non-composed types.
    pub const fn available_animation_actions(&self) -> Option<&'static [&'static str]> {
        match self {
            Self::Enemy(EnemyType::Mosquiton) => Some(mosquiton::GALLERY_ACTIONS),
            Self::Enemy(EnemyType::Spidey) => Some(spidey::GALLERY_ACTIONS),
            _ => None,
        }
    }

    /// Default animation action for preview. `None` for non-composed types.
    pub const fn default_animation_action(&self) -> Option<&'static str> {
        match self {
            Self::Enemy(EnemyType::Mosquiton) => Some(mosquiton::ACTION_IDLE_FLY),
            Self::Enemy(EnemyType::Spidey) => Some(spidey::ACTION_IDLE),
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
