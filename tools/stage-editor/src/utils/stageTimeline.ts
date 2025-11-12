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
    return step.Stop.max_duration ?? 999999
  }

  if ("Movement" in step) {
    // Movement steps: calculate time based on distance and speed
    const movement = step.Movement
    if (!endPos) return 0

    const dx = endPos[0] - startPos[0]
    const dy = endPos[1] - startPos[1]
    const distance = Math.sqrt(dx * dx + dy * dy)
    return distance / movement.base_speed
  }

  if ("Cinematic" in step) {
    // Cinematics: for now, use a fixed duration (can be enhanced later)
    return 5 // Default 5 seconds for cinematics
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
  const startCoords = stageData.start_coordinates ?? [0, 0]

  for (let i = 0; i < stageData.steps.length; i++) {
    const step = stageData.steps[i]

    markers.push({
      time: cumulativeTime,
      type:
        "Stop" in step ? "Stop" : "Movement" in step ? "Movement" : "Cinematic",
      index: i,
    })

    // Find the current position by looking back through all previous steps
    // to find the last Movement step
    let currentPos: [number, number] = startCoords
    for (let j = i - 1; j >= 0; j--) {
      const prevStep = stageData.steps[j]
      if ("Movement" in prevStep) {
        currentPos = (
          prevStep as { Movement: { coordinates: [number, number] } }
        ).Movement.coordinates
        break
      }
    }

    const nextPos =
      "Movement" in step
        ? (step as { Movement: { coordinates: [number, number] } }).Movement
            .coordinates
        : currentPos

    const duration = getStepDuration(step, currentPos, nextPos)
    cumulativeTime += duration
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

  // Find which step we're in
  let currentStepIndex = 0
  for (let i = markers.length - 1; i >= 0; i--) {
    if (time >= markers[i].time) {
      currentStepIndex = i
      break
    }
  }

  const marker = markers[currentStepIndex]
  const step = stageData.steps[marker.index]
  const stepStartTime = marker.time
  const elapsedInStep = time - stepStartTime

  // Find the starting position for this step by looking back through all
  // previous steps to find the last Movement step
  let stepStartPos: [number, number] = startCoords
  for (let i = marker.index - 1; i >= 0; i--) {
    const prevStep = stageData.steps[i]
    if ("Movement" in prevStep) {
      stepStartPos = (
        prevStep as { Movement: { coordinates: [number, number] } }
      ).Movement.coordinates
      break
    }
  }

  // Calculate position based on step type
  if ("Stop" in step) {
    // Camera stays at the same position
    return { x: stepStartPos[0], y: stepStartPos[1] }
  }

  if ("Movement" in step) {
    const movement = (
      step as {
        Movement: { coordinates: [number, number]; base_speed: number }
      }
    ).Movement
    const targetPos = movement.coordinates

    // Calculate how far we've moved in this step
    const dx = targetPos[0] - stepStartPos[0]
    const dy = targetPos[1] - stepStartPos[1]
    const distance = Math.sqrt(dx * dx + dy * dy)

    if (distance === 0) {
      return { x: stepStartPos[0], y: stepStartPos[1] }
    }

    const travelTime = distance / movement.base_speed
    const progress = Math.min(1, elapsedInStep / travelTime)

    return {
      x: stepStartPos[0] + dx * progress,
      y: stepStartPos[1] + dy * progress,
    }
  }

  if ("Cinematic" in step) {
    // For cinematics, camera stays at the previous position
    return { x: stepStartPos[0], y: stepStartPos[1] }
  }

  return { x: stepStartPos[0], y: stepStartPos[1] }
}
