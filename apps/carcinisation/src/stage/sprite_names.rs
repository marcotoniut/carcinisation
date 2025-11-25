//! Sprite base name mappings for all spawnable entities.
//!
//! This module provides the single source of truth for sprite file naming conventions.
//! The mappings are exported to TypeScript via ts-rs for consistent sprite loading.

use super::{
    data::{ObjectType, PickupType},
    destructible::components::DestructibleType,
    enemy::entity::EnemyType,
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "derive-ts")]
use ts_rs::TS;

/// Sprite base name mapping for enemies
#[cfg_attr(feature = "derive-ts", derive(TS))]
#[cfg_attr(feature = "derive-ts", ts(export))]
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct EnemySpriteName {
    pub enemy_type: EnemyType,
    pub base_name: String,
}

/// Sprite base name mapping for objects
#[cfg_attr(feature = "derive-ts", derive(TS))]
#[cfg_attr(feature = "derive-ts", ts(export))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ObjectSpriteName {
    pub object_type: ObjectType,
    pub base_name: String,
}

/// Sprite base name mapping for pickups
#[cfg_attr(feature = "derive-ts", derive(TS))]
#[cfg_attr(feature = "derive-ts", ts(export))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PickupSpriteName {
    pub pickup_type: PickupType,
    pub base_name: String,
}

/// Sprite base name mapping for destructibles
#[cfg_attr(feature = "derive-ts", derive(TS))]
#[cfg_attr(feature = "derive-ts", ts(export))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DestructibleSpriteName {
    pub destructible_type: DestructibleType,
    pub base_name: String,
}

/// Complete sprite name registry exported to TypeScript
#[cfg_attr(feature = "derive-ts", derive(TS))]
#[cfg_attr(feature = "derive-ts", ts(export))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SpriteNameRegistry {
    pub enemies: Vec<EnemySpriteName>,
    pub objects: Vec<ObjectSpriteName>,
    pub pickups: Vec<PickupSpriteName>,
    pub destructibles: Vec<DestructibleSpriteName>,
}

/// TODO should probably be a trait of the enemy itself.
impl SpriteNameRegistry {
    /// Creates the complete registry with all sprite base names
    pub fn new() -> Self {
        Self {
            enemies: vec![
                EnemySpriteName {
                    enemy_type: EnemyType::Mosquito,
                    base_name: "mosquito".to_string(),
                },
                EnemySpriteName {
                    enemy_type: EnemyType::Spidey,
                    base_name: "spidey".to_string(),
                },
                EnemySpriteName {
                    enemy_type: EnemyType::Tardigrade,
                    base_name: "tardigrade".to_string(),
                },
                EnemySpriteName {
                    enemy_type: EnemyType::Marauder,
                    base_name: "marauder".to_string(),
                },
                EnemySpriteName {
                    enemy_type: EnemyType::Spidomonsta,
                    base_name: "spidomonsta".to_string(),
                },
                EnemySpriteName {
                    enemy_type: EnemyType::Kyle,
                    base_name: "kyle".to_string(),
                },
            ],
            objects: vec![
                ObjectSpriteName {
                    object_type: ObjectType::BenchBig,
                    base_name: "bench_big".to_string(),
                },
                ObjectSpriteName {
                    object_type: ObjectType::BenchSmall,
                    base_name: "bench_small".to_string(),
                },
                ObjectSpriteName {
                    object_type: ObjectType::Fibertree,
                    base_name: "fiber_tree".to_string(),
                },
                ObjectSpriteName {
                    object_type: ObjectType::RugparkSign,
                    base_name: "rugpark_sign".to_string(),
                },
            ],
            pickups: vec![
                PickupSpriteName {
                    pickup_type: PickupType::SmallHealthpack,
                    base_name: "health_4".to_string(),
                },
                PickupSpriteName {
                    pickup_type: PickupType::BigHealthpack,
                    base_name: "health_6".to_string(),
                },
            ],
            destructibles: vec![
                DestructibleSpriteName {
                    destructible_type: DestructibleType::Lamp,
                    base_name: "lamp".to_string(),
                },
                DestructibleSpriteName {
                    destructible_type: DestructibleType::Trashcan,
                    base_name: "trashcan".to_string(),
                },
                DestructibleSpriteName {
                    destructible_type: DestructibleType::Crystal,
                    base_name: "crystal".to_string(),
                },
                DestructibleSpriteName {
                    destructible_type: DestructibleType::Mushroom,
                    base_name: "mushroom".to_string(),
                },
            ],
        }
    }
}

impl Default for SpriteNameRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn registry_matches_sprite_base_name_methods() {
        let registry = SpriteNameRegistry::new();

        let enemy_map: HashMap<_, _> = registry
            .enemies
            .iter()
            .map(|entry| (entry.enemy_type, entry.base_name.as_str()))
            .collect();
        for enemy in [
            EnemyType::Mosquito,
            EnemyType::Spidey,
            EnemyType::Tardigrade,
            EnemyType::Marauder,
            EnemyType::Spidomonsta,
            EnemyType::Kyle,
        ] {
            assert_eq!(
                enemy.sprite_base_name(),
                enemy_map.get(&enemy).copied().unwrap(),
                "Enemy base name mismatch for {:?}",
                enemy
            );
        }

        let object_map: HashMap<_, _> = registry
            .objects
            .iter()
            .map(|entry| (entry.object_type, entry.base_name.as_str()))
            .collect();
        for object in [
            ObjectType::BenchBig,
            ObjectType::BenchSmall,
            ObjectType::Fibertree,
            ObjectType::RugparkSign,
        ] {
            assert_eq!(
                object.sprite_base_name(),
                object_map.get(&object).copied().unwrap(),
                "Object base name mismatch for {:?}",
                object
            );
        }

        let pickup_map: HashMap<_, _> = registry
            .pickups
            .iter()
            .map(|entry| (entry.pickup_type, entry.base_name.as_str()))
            .collect();
        for pickup in [PickupType::SmallHealthpack, PickupType::BigHealthpack] {
            assert_eq!(
                pickup.sprite_base_name(),
                pickup_map.get(&pickup).copied().unwrap(),
                "Pickup base name mismatch for {:?}",
                pickup
            );
        }

        let destructible_map: HashMap<_, _> = registry
            .destructibles
            .iter()
            .map(|entry| (entry.destructible_type, entry.base_name.as_str()))
            .collect();
        for (destructible, expected_base) in [
            (DestructibleType::Lamp, "lamp"),
            (DestructibleType::Trashcan, "trashcan"),
            (DestructibleType::Crystal, "crystal"),
            (DestructibleType::Mushroom, "mushroom"),
        ] {
            assert_eq!(
                expected_base,
                destructible_map.get(&destructible).copied().unwrap(),
                "Destructible base name mismatch for {:?}",
                destructible
            );
        }
    }
}
