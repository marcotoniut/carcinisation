import { Assets, Sprite } from "pixi.js"
import type { SpawnId } from "@/state/store"
import type { Depth } from "@/types/generated/Depth"
import type { StageData } from "@/types/generated/StageData"
import { createFallbackTexture } from "./createFallbackTexture"
import { getCachedTexture } from "./pixiCache"
import {
  getDestructibleSpritePaths,
  getEnemySpritePaths,
  getObjectSpritePath,
  getPickupSpritePath,
} from "./spriteMapping"

// Depth enum values: Nine=9 (back) to Zero=0 (front)
// In PixiJS, higher zIndex renders on top, so we invert the depth values
// Zero (0) should be highest zIndex (100) to render in front
// Nine (9) should be lowest zIndex (10) to render in back
const DEPTH_Z_INDEX: Record<Depth, number> = {
  Zero: 100, // Front-most layer
  One: 90,
  Two: 80,
  Three: 70,
  Four: 60,
  Five: 50,
  Six: 40,
  Seven: 30,
  Eight: 20,
  Nine: 10, // Back-most layer
}

function getDepthZIndex(depth: Depth): number {
  return DEPTH_Z_INDEX[depth]
}

type WorldToScreen = (world: { x: number; y: number }) => {
  x: number
  y: number
}

export type SpawnSpriteData = {
  sprite: Sprite
  spawnId: SpawnId
  elapsed: number // When this spawn appears in seconds
  order: number
}

async function loadTexture(pathOrPaths: string | string[]): Promise<Sprite> {
  const paths = Array.isArray(pathOrPaths) ? pathOrPaths : [pathOrPaths]

  for (const path of paths) {
    const resolvedPath = path.startsWith("/") ? path : `/${path}`
    const cached = getCachedTexture(resolvedPath)
    if (cached) return new Sprite(cached)
    try {
      const texture = await Assets.load(resolvedPath)
      return new Sprite(texture)
    } catch (_error) {
      console.warn(
        `Sprite not found: ${resolvedPath}, trying next variant or using fallback`,
      )
    }
  }

  console.warn(
    `All sprite variants missing for paths: ${paths.join(
      ", ",
    )}, using fallback texture`,
  )
  // Return MissingNo-style fallback texture
  const fallbackTexture = createFallbackTexture()
  return new Sprite(fallbackTexture)
}

export async function renderSpawns(
  stageData: StageData,
  worldToScreen: WorldToScreen,
  entityAnimations: Map<string, string>,
): Promise<SpawnSpriteData[]> {
  const spawnSprites: SpawnSpriteData[] = []

  if (!stageData.spawns) {
    return spawnSprites
  }

  // Track spawn counts per type for indexing
  const spawnCounts = { enemy: 0, object: 0, pickup: 0, destructible: 0 }

  for (const spawn of stageData.spawns) {
    if ("Enemy" in spawn) {
      const enemy = spawn.Enemy
      const spawnId: SpawnId = { type: "enemy", index: spawnCounts.enemy++ }
      const animationKey = `${spawnId.type}:${spawnId.index}`
      const animation = entityAnimations.get(animationKey) ?? "idle"

      const spritePaths = getEnemySpritePaths(enemy.enemy_type, animation)
      const sprite = await loadTexture(spritePaths)

      const screenPos = worldToScreen({
        x: enemy.coordinates[0],
        y: enemy.coordinates[1],
      })
      sprite.position.set(screenPos.x, screenPos.y)
      sprite.anchor.set(0.5, 1)
      sprite.zIndex = getDepthZIndex(enemy.depth)
      sprite.interactive = true

      spawnSprites.push({
        sprite,
        spawnId,
        elapsed: enemy.elapsed,
        order: spawnSprites.length,
      })
    } else if ("Object" in spawn) {
      const object = spawn.Object
      const spawnId: SpawnId = { type: "object", index: spawnCounts.object++ }

      const spritePath = getObjectSpritePath(object.object_type)
      const sprite = await loadTexture(spritePath)

      const screenPos = worldToScreen({
        x: object.coordinates[0],
        y: object.coordinates[1],
      })
      sprite.position.set(screenPos.x, screenPos.y)
      sprite.anchor.set(0.5, 1)
      sprite.zIndex = getDepthZIndex(object.depth)
      sprite.interactive = true

      spawnSprites.push({
        sprite,
        spawnId,
        elapsed: 0, // Objects have no spawn time
        order: spawnSprites.length,
      })
    } else if ("Pickup" in spawn) {
      const pickup = spawn.Pickup
      const spawnId: SpawnId = { type: "pickup", index: spawnCounts.pickup++ }

      const spritePath = getPickupSpritePath(pickup.pickup_type)
      const sprite = await loadTexture(spritePath)

      const screenPos = worldToScreen({
        x: pickup.coordinates[0],
        y: pickup.coordinates[1],
      })
      sprite.position.set(screenPos.x, screenPos.y)
      sprite.anchor.set(0.5, 1)
      sprite.zIndex = getDepthZIndex(pickup.depth)
      sprite.interactive = true

      spawnSprites.push({
        sprite,
        spawnId,
        elapsed: pickup.elapsed,
        order: spawnSprites.length,
      })
    } else if ("Destructible" in spawn) {
      const destructible = spawn.Destructible
      const spawnId: SpawnId = {
        type: "destructible",
        index: spawnCounts.destructible++,
      }
      const animationKey = `${spawnId.type}:${spawnId.index}`
      const animation = entityAnimations.get(animationKey) ?? "base"

      const spritePaths = getDestructibleSpritePaths(
        destructible.destructible_type,
        animation,
      )
      const sprite = await loadTexture(spritePaths)

      const screenPos = worldToScreen({
        x: destructible.coordinates[0],
        y: destructible.coordinates[1],
      })
      sprite.position.set(screenPos.x, screenPos.y)
      sprite.anchor.set(0.5, 1)
      sprite.zIndex = getDepthZIndex(destructible.depth)
      sprite.interactive = true

      spawnSprites.push({
        sprite,
        spawnId,
        elapsed: 0, // Destructibles have no spawn time
        order: spawnSprites.length,
      })
    }
  }

  return spawnSprites
}

export function updateSpawnOpacity(
  spawnSprites: SpawnSpriteData[],
  currentTime: number,
) {
  for (const { sprite, elapsed } of spawnSprites) {
    // Spawns appear at their elapsed time
    // Show at 50% opacity if not yet spawned, 100% if spawned
    sprite.alpha = currentTime >= elapsed ? 1.0 : 0.5
  }
}
