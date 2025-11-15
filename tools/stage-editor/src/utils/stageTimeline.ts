import type { StageData } from "@/types/generated/StageData"

export interface StepMarker {
  time: number // Time in seconds when this step starts
  type: "Stop" | "Movement" | "Cinematic"
  index: number
}

export interface CameraPosition {
  x: number
  y: number
}

const DEFAULT_STOP_DURATION = 999999

/**
 * Calculate the duration of a single step
 */
function getStepDuration(
  step: StageData["steps"][number],
  startPos: [number, number],
  endPos: [number, number] | null,
): number {
  if ("Stop" in step) {
    // Stop steps have a max_duration (or infinite if null)
    return step.Stop.max_duration ?? DEFAULT_STOP_DURATION
  }

  if ("Movement" in step) {
    const movement = step.Movement
    if (!endPos) return 0

    const dx = endPos[0] - startPos[0]
    const dy = endPos[1] - startPos[1]
    const distance = Math.sqrt(dx * dx + dy * dy)
    return distance / movement.base_speed
  }

  if ("Cinematic" in step) {
    // Cinematics: for now, use a fixed duration (can be enhanced later)
    return 5
  }

  return 0
}

/**
 * Build step markers with cumulative times
 */
export function getStepMarkers(stageData: StageData | null): StepMarker[] {
  if (!stageData || !stageData.steps || stageData.steps.length === 0) {
    return []
  }

  const markers: StepMarker[] = []
  let cumulativeTime = 0
  let currentPos: [number, number] = [
    stageData.start_coordinates[0],
    stageData.start_coordinates[1],
  ]

  for (let i = 0; i < stageData.steps.length; i++) {
    const step = stageData.steps[i]
    const nextPos = "Movement" in step ? step.Movement.coordinates : currentPos

    markers.push({
      time: cumulativeTime,
      type:
        "Stop" in step ? "Stop" : "Movement" in step ? "Movement" : "Cinematic",
      index: i,
    })

    const duration = getStepDuration(step, currentPos, nextPos)
    cumulativeTime += duration

    if ("Movement" in step) {
      currentPos = step.Movement.coordinates
    }
  }

  return markers
}

/**
 * Get total timeline duration
 */
export function getTotalDuration(markers: StepMarker[]): number {
  if (markers.length === 0) return 0
  return markers[markers.length - 1].time + 10 // Add 10s buffer for the last step
}

/**
 * Calculate camera position at a given time
 */
export function getCameraPosition(
  stageData: StageData | null,
  time: number,
  markers: StepMarker[],
): CameraPosition {
  const startCoords = stageData?.start_coordinates ?? [0, 0]

  if (!stageData || markers.length === 0) {
    return { x: startCoords[0], y: startCoords[1] }
  }

  let currentPos: [number, number] = [startCoords[0], startCoords[1]]
  let elapsed = 0

  for (let i = 0; i < stageData.steps.length; i++) {
    const step = stageData.steps[i]
    const nextPos = "Movement" in step ? step.Movement.coordinates : currentPos
    const duration = getStepDuration(step, currentPos, nextPos)
    const stepEnd = elapsed + duration

    if (time < stepEnd) {
      if ("Movement" in step) {
        if (duration === 0) {
          return { x: nextPos[0], y: nextPos[1] }
        }

        const dx = nextPos[0] - currentPos[0]
        const dy = nextPos[1] - currentPos[1]
        const progress = Math.min(1, Math.max(0, (time - elapsed) / duration))

        return {
          x: currentPos[0] + dx * progress,
          y: currentPos[1] + dy * progress,
        }
      }

      return { x: currentPos[0], y: currentPos[1] }
    }

    if ("Movement" in step) {
      currentPos = step.Movement.coordinates
    }

    elapsed = stepEnd
  }

  return { x: currentPos[0], y: currentPos[1] }
}
