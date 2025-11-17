import { Texture } from "pixi.js"

/**
 * Creates a MissingNo-style fallback texture for missing sprites
 * A 6x6 purple/black checkerboard pattern
 */
export function createFallbackTexture(): Texture {
  const size = 6
  const canvas = document.createElement("canvas")
  canvas.width = size
  canvas.height = size

  const ctx = canvas.getContext("2d")
  if (!ctx) {
    return Texture.EMPTY
  }

  // MissingNo colors: purple and black checkerboard
  const color1 = "#9D4EDD" // Purple
  const color2 = "#000000" // Black

  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) {
      // Checkerboard pattern
      const isEven = (x + y) % 2 === 0
      ctx.fillStyle = isEven ? color1 : color2
      ctx.fillRect(x, y, 1, 1)
    }
  }

  return Texture.from(canvas)
}
