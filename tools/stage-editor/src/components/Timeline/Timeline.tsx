import { useMemo } from "react"
import { useEditorStore } from "../../state/store"
import { getStepMarkers, getTotalDuration } from "../../utils/stageTimeline"
import "./Timeline.css"

export function Timeline() {
  const { parsedData, timelinePosition, setTimelinePosition } = useEditorStore()

  // Calculate step markers and duration
  const stepMarkers = useMemo(() => getStepMarkers(parsedData), [parsedData])
  const maxDuration = useMemo(
    () => getTotalDuration(stepMarkers),
    [stepMarkers],
  )

  const getStopTooltip = (
    marker: ReturnType<typeof getStepMarkers>[number],
  ) => {
    if (!parsedData || marker.type !== "Stop") return ""

    const step = parsedData.steps[marker.index]
    if (!("Stop" in step)) return ""

    const stopData = step.Stop
    const enemyCount = stopData.spawns.filter((s) => "Enemy" in s).length
    const objectCount = stopData.spawns.filter((s) => "Object" in s).length
    const destructibleCount = stopData.spawns.filter(
      (s) => "Destructible" in s,
    ).length
    const pickupCount = stopData.spawns.filter((s) => "Pickup" in s).length

    const duration = stopData.max_duration ?? "∞"
    const durationStr =
      typeof duration === "number" ? `${duration.toFixed(1)}s` : duration

    const parts = [
      `Stop at ${marker.time.toFixed(1)}s`,
      `Duration: ${durationStr}`,
    ]

    if (enemyCount > 0) parts.push(`Enemies: ${enemyCount}`)
    if (objectCount > 0) parts.push(`Objects: ${objectCount}`)
    if (destructibleCount > 0) parts.push(`Destructibles: ${destructibleCount}`)
    if (pickupCount > 0) parts.push(`Pickups: ${pickupCount}`)

    if (stopData.kill_all) parts.push("Kill All")
    if (stopData.kill_boss) parts.push("Kill Boss")

    return parts.join("\n")
  }

  const handleSliderChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    const value = Number.parseFloat(event.target.value)

    // Snap to nearest Stop marker when within 0.5s
    const snapThreshold = 0.5
    for (const marker of stepMarkers) {
      if (
        marker.type === "Stop" &&
        Math.abs(value - marker.time) < snapThreshold
      ) {
        setTimelinePosition(marker.time)
        return
      }
    }

    setTimelinePosition(value)
  }

  return (
    <div className="timeline panel">
      <div className="timeline-content">
        <div className="timeline-controls">
          <label htmlFor="timeline-slider">
            <header className="timeline-header">
              <span>Timeline</span>
              <span>{timelinePosition.toFixed(1)}s</span>
            </header>
            <div className="timeline-slider-container">
              <input
                id="timeline-slider"
                type="range"
                min="0"
                max={maxDuration || 100}
                step="0.1"
                value={timelinePosition}
                onChange={handleSliderChange}
                className="timeline-slider"
              />
              <div className="timeline-markers">
                {stepMarkers.map((marker) => {
                  if (marker.type !== "Stop") return null
                  const percentage =
                    maxDuration > 0 ? (marker.time / maxDuration) * 100 : 0
                  return (
                    <div
                      key={`stop-${marker.index}-${marker.time}`}
                      className="timeline-marker timeline-marker-stop"
                      style={{ left: `${percentage}%` }}
                      title={getStopTooltip(marker)}
                    />
                  )
                })}
              </div>
            </div>
          </label>
        </div>
      </div>
    </div>
  )
}
