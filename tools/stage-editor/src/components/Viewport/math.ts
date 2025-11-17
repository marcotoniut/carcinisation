import type { Point } from "pixi.js"

export function distanceSquaredPoints(a: Point, b: Point): number {
  const dx = a.x - b.x
  const dy = a.y - b.y
  return dx * dx + dy * dy
}

export function calculateDistance(touches: React.TouchList): number {
  if (touches.length < 2) return 0
  const [a, b] = [touches[0], touches[1]]
  const dx = a.clientX - b.clientX
  const dy = a.clientY - b.clientY
  return Math.sqrt(dx * dx + dy * dy)
}
