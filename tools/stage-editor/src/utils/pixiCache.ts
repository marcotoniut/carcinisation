import type { Texture } from "pixi.js"
import { Assets, Sprite } from "pixi.js"

/**
 * Return a cached texture if present, otherwise undefined.
 */
export function getCachedTexture(path: string): Texture | undefined {
  const cacheHolder = Assets as unknown as {
    cache?: {
      has: (key: string) => boolean
      get: (key: string) => Texture | undefined
    }
  }
  const cache = cacheHolder.cache
  if (cache?.has?.(path)) {
    return cache.get(path)
  }
  return undefined
}

/**
 * Create a Sprite from cache if available.
 */
export function spriteFromCache(path: string): Sprite | undefined {
  const cached = getCachedTexture(path)
  return cached ? new Sprite(cached) : undefined
}
