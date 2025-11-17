/**
 * Single source of truth for sprite base names.
 * These mappings correspond to actual sprite filenames and must match the Rust implementations.
 *
 * Rust source of truth:
 * - ObjectType: apps/carcinisation/src/stage/data.rs
 * - PickupType: apps/carcinisation/src/stage/data.rs
 * - EnemyType: apps/carcinisation/src/stage/enemy/entity.rs
 * - DestructibleType: apps/carcinisation/src/stage/destructible/components/mod.rs
 */

import type { DestructibleType } from "../types/generated/DestructibleType"
import type { EnemyType } from "../types/generated/EnemyType"
import type { ObjectType } from "../types/generated/ObjectType"
import type { PickupType } from "../types/generated/PickupType"

/**
 * Maps enemy types to their sprite base names
 */
export const ENEMY_SPRITE_NAMES: Record<EnemyType, string> = {
  Mosquito: "mosquito",
  Spidey: "spidey",
  Tardigrade: "tardigrade",
  Marauder: "marauder",
  Spidomonsta: "spidomonsta",
  Kyle: "kyle",
}

/**
 * Maps object types to their sprite base names
 */
export const OBJECT_SPRITE_NAMES: Record<ObjectType, string> = {
  BenchBig: "bench_big",
  BenchSmall: "bench_small",
  Fibertree: "fiber_tree",
  RugparkSign: "rugpark_sign",
}

/**
 * Maps pickup types to their sprite base names
 */
export const PICKUP_SPRITE_NAMES: Record<PickupType, string> = {
  SmallHealthpack: "health_4",
  BigHealthpack: "health_6",
}

/**
 * Maps destructible types to their sprite base names
 */
export const DESTRUCTIBLE_SPRITE_NAMES: Record<DestructibleType, string> = {
  Lamp: "lamp",
  Trashcan: "trashcan",
  Crystal: "crystal",
  Mushroom: "mushroom",
}
