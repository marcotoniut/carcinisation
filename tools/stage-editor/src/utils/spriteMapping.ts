import type { DestructibleType } from "@/types/generated/DestructibleType"
import type { EnemyType } from "@/types/generated/EnemyType"
import type { ObjectType } from "@/types/generated/ObjectType"
import type { PickupType } from "@/types/generated/PickupType"
import {
  DESTRUCTIBLE_SPRITE_NAMES,
  ENEMY_SPRITE_NAMES,
  OBJECT_SPRITE_NAMES,
  PICKUP_SPRITE_NAMES,
} from "./spriteNames"

/**
 * Dynamically constructs sprite paths using the single source of truth sprite names.
 * Falls back to MissingNo texture if the sprite doesn't exist (handled by renderSpawns).
 */

// Common sprite size variants to try in order of preference
const SPRITE_SIZE_VARIANTS = [3, 4, 5, 6, 7, 8]

export function getEnemySpritePaths(
  enemyType: EnemyType,
  animation = "idle",
): string[] {
  const baseName = ENEMY_SPRITE_NAMES[enemyType]
  return SPRITE_SIZE_VARIANTS.map(
    (size) => `/sprites/enemies/${baseName}_${animation}_${size}.px_sprite.png`,
  )
}

export function getEnemySpritePath(
  enemyType: EnemyType,
  animation = "idle",
): string {
  // Default to the first candidate variant; loaders may try others.
  return getEnemySpritePaths(enemyType, animation)[0]
}

export function getObjectSpritePath(objectType: ObjectType): string {
  const baseName = OBJECT_SPRITE_NAMES[objectType]
  return `/sprites/objects/${baseName}.px_sprite.png`
}

export function getPickupSpritePath(pickupType: PickupType): string {
  const baseName = PICKUP_SPRITE_NAMES[pickupType]
  return `/sprites/pickups/${baseName}.px_sprite.png`
}

export function getDestructibleSpritePath(
  destructibleType: DestructibleType,
  animation = "base",
): string {
  const _baseName = DESTRUCTIBLE_SPRITE_NAMES[destructibleType]
  // Default to the first candidate variant; loaders may try others.
  return getDestructibleSpritePaths(destructibleType, animation)[0]
}

export function getDestructibleSpritePaths(
  destructibleType: DestructibleType,
  animation = "base",
): string[] {
  const baseName = DESTRUCTIBLE_SPRITE_NAMES[destructibleType]
  return SPRITE_SIZE_VARIANTS.map(
    (size) => `/sprites/objects/${baseName}_${animation}_${size}.px_sprite.png`,
  )
}
