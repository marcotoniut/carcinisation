import * as Slider from "@radix-ui/react-slider"
import * as Tooltip from "@radix-ui/react-tooltip"
import { useMemo } from "react"
import { useEditorStore } from "../../state/store"
import { getStepMarkers, getTotalDuration } from "../../utils/stageTimeline"
import * as styles from "./Timeline.css"

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

  const handleSliderChange = (value: number[]) => {
    const newValue = value[0]

    // Snap to nearest Stop marker when within 0.5s
    const snapThreshold = 0.5
    for (const marker of stepMarkers) {
      if (
        marker.type === "Stop" &&
        Math.abs(newValue - marker.time) < snapThreshold
      ) {
        setTimelinePosition(marker.time)
        return
      }
    }

    setTimelinePosition(newValue)
  }

  return (
    <Tooltip.Provider>
      <div className={`${styles.timeline} panel`}>
        <div className={styles.timelineContent}>
          <div className={styles.timelineControls}>
            <div className={styles.timelineLabel}>
              <header className={styles.timelineHeader}>
                <span>Timeline</span>
                <span>{timelinePosition.toFixed(1)}s</span>
              </header>
              <div className={styles.timelineSliderContainer}>
                <Slider.Root
                  className={styles.sliderRoot}
                  min={0}
                  max={maxDuration || 100}
                  step={0.1}
                  value={[timelinePosition]}
                  onValueChange={handleSliderChange}
                  aria-label="Timeline position"
                >
                  <Slider.Track className={styles.sliderTrack}>
                    <Slider.Range className={styles.sliderRange} />
                    {stepMarkers.map((marker) => {
                      if (marker.type !== "Stop") return null
                      const percentage =
                        maxDuration > 0 ? (marker.time / maxDuration) * 100 : 0
                      const tooltip = getStopTooltip(marker)
                      const isPassed = timelinePosition >= marker.time
                      const markerClassName = `${styles.timelineMarker} ${styles.timelineMarkerStop} ${
                        isPassed ? styles.timelineMarkerPassed : ""
                      }`

                      return (
                        <Tooltip.Root
                          key={`stop-${marker.index}-${marker.time}`}
                        >
                          <Tooltip.Trigger asChild>
                            <button
                              type="button"
                              className={markerClassName}
                              style={{ left: `${percentage}%` }}
                              onClick={() => setTimelinePosition(marker.time)}
                            />
                          </Tooltip.Trigger>
                          <Tooltip.Portal>
                            <Tooltip.Content
                              side="top"
                              style={{
                                backgroundColor: "rgba(0, 0, 0, 0.9)",
                                color: "white",
                                padding: "8px 12px",
                                borderRadius: "4px",
                                fontSize: "12px",
                                whiteSpace: "pre-line",
                                maxWidth: "300px",
                                zIndex: 9999,
                              }}
                            >
                              {tooltip}
                              <Tooltip.Arrow
                                style={{ fill: "rgba(0, 0, 0, 0.9)" }}
                              />
                            </Tooltip.Content>
                          </Tooltip.Portal>
                        </Tooltip.Root>
                      )
                    })}
                  </Slider.Track>
                  <Slider.Thumb className={styles.sliderThumb} />
                </Slider.Root>
                {/* <div className={styles.timelineMarkers}></div> */}
              </div>
            </div>
          </div>
        </div>
      </div>
    </Tooltip.Provider>
  )
}
