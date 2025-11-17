export type WorldPoint = { x: number; y: number }
export type ScreenPoint = { x: number; y: number }

/**
 * Factory so we can adjust origin offsets/scaling later.
 */
export const makeWorldToScreen = (originYOffset = 0) => {
  return (world: WorldPoint): ScreenPoint => ({
    x: world.x,
    // World is Y-up while Pixi is Y-down; invert and offset so y=0 aligns with
    // the stage origin (background bottom-left).
    y: originYOffset - world.y,
  })
}
