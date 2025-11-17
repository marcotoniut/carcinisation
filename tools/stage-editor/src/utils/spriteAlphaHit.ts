import type { Sprite, Texture } from "pixi.js"
import { Point } from "pixi.js"

type AlphaData = {
  data: Uint8ClampedArray
  width: number
  height: number
}

const alphaCache = new WeakMap<Texture, AlphaData>()

function extractSource(
  texture: Texture,
): HTMLImageElement | HTMLCanvasElement | ImageBitmap | null {
  const source = (
    texture.source as { resource?: { source?: unknown } } | undefined
  )?.resource?.source
  if (!source) return null
  if (
    source instanceof HTMLImageElement ||
    source instanceof HTMLCanvasElement ||
    source instanceof ImageBitmap
  ) {
    return source
  }
  return null
}

function getAlphaData(texture: Texture): AlphaData | null {
  const cached = alphaCache.get(texture)
  if (cached) return cached

  const source = extractSource(texture)
  if (!source) return null

  const canvas = document.createElement("canvas")
  canvas.width = source.width
  canvas.height = source.height
  const ctx = canvas.getContext("2d")
  if (!ctx) return null

  ctx.drawImage(source, 0, 0)
  const imageData = ctx.getImageData(0, 0, source.width, source.height)
  const alphaData: AlphaData = {
    data: imageData.data,
    width: source.width,
    height: source.height,
  }
  alphaCache.set(texture, alphaData)
  return alphaData
}

/**
 * Returns true if the global point hits a non-transparent pixel of the sprite.
 */
export function isSpriteAlphaHit(
  sprite: Sprite,
  global: { x: number; y: number },
): boolean {
  const texture = sprite.texture
  const alphaData = getAlphaData(texture)
  if (!alphaData) return false

  const local = new Point(global.x, global.y)
  sprite.worldTransform.applyInverse(local, local)

  const frame = texture.frame
  const width = frame.width
  const height = frame.height
  const anchorX = sprite.anchor?.x ?? 0
  const anchorY = sprite.anchor?.y ?? 0

  const u = local.x + anchorX * width
  const v = local.y + anchorY * height
  if (u < 0 || v < 0 || u >= width || v >= height) return false

  const x = Math.floor(frame.x + u)
  const y = Math.floor(frame.y + v)

  if (x < 0 || y < 0 || x >= alphaData.width || y >= alphaData.height) {
    return false
  }

  const idx = (y * alphaData.width + x) * 4 + 3
  const alpha = alphaData.data[idx]
  return alpha > 0
}
