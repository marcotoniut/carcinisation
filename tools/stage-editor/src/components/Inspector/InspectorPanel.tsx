import type { StageData } from "@/types/generated/StageData"
import type { SpawnId } from "../../state/store"
import { useEditorStore } from "../../state/store"
import * as styles from "./InspectorPanel.css"

export function InspectorPanel() {
  const parsedData = useEditorStore((state) => state.parsedData)
  const selectedSpawn = useEditorStore((state) => state.selectedSpawn)
  const getEntityAnimation = useEditorStore((state) => state.getEntityAnimation)
  const setEntityAnimation = useEditorStore((state) => state.setEntityAnimation)

  if (!selectedSpawn || !parsedData) {
    return (
      <div className={`${styles.inspector} panel`}>
        <div className="panel-header">Inspector</div>
        <div className={styles.inspectorEmpty}>
          Select an entity to view properties
        </div>
      </div>
    )
  }

  const spawnData = getSpawnData(parsedData, selectedSpawn)
  if (!spawnData) {
    return (
      <div className={`${styles.inspector} panel`}>
        <div className="panel-header">Inspector</div>
        <div className={styles.inspectorEmpty}>Invalid selection</div>
      </div>
    )
  }

  const currentAnimation = getEntityAnimation(selectedSpawn)
  const availableAnimations = getAvailableAnimations(selectedSpawn)

  return (
    <div className={`${styles.inspector} panel`}>
      <div className="panel-header">
        Inspector - {selectedSpawn.type} #{selectedSpawn.index}
      </div>
      <div className={styles.inspectorContent}>
        {/* Type */}
        <div className={styles.propertyGroup}>
          <span className={styles.propertyLabel}>Type</span>
          <div className={styles.propertyValue}>
            {spawnData.typeName ?? "Unknown"}
          </div>
        </div>

        {/* Coordinates */}
        <div className={styles.propertyGroup}>
          <span className={styles.propertyLabel}>Coordinates</span>
          <div className={styles.propertyValue}>
            x: {spawnData.coordinates[0]}, y: {spawnData.coordinates[1]}
          </div>
        </div>

        {/* Depth */}
        {spawnData.depth && (
          <div className={styles.propertyGroup}>
            <span className={styles.propertyLabel}>Depth Layer</span>
            <div className={styles.propertyValue}>{spawnData.depth}</div>
          </div>
        )}

        {/* Elapsed (spawn time) */}
        {spawnData.elapsed !== undefined && (
          <div className={styles.propertyGroup}>
            <span className={styles.propertyLabel}>Spawn Time</span>
            <div className={styles.propertyValue}>{spawnData.elapsed}s</div>
          </div>
        )}

        {/* Speed (enemies only) */}
        {spawnData.speed !== undefined && (
          <div className={styles.propertyGroup}>
            <span className={styles.propertyLabel}>Speed</span>
            <div className={styles.propertyValue}>{spawnData.speed}</div>
          </div>
        )}

        {/* Health (destructibles only) */}
        {spawnData.health !== undefined && (
          <div className={styles.propertyGroup}>
            <span className={styles.propertyLabel}>Health</span>
            <div className={styles.propertyValue}>{spawnData.health}</div>
          </div>
        )}

        {/* Animation Selector */}
        {availableAnimations.length > 0 && (
          <div className={styles.propertyGroup}>
            <div className={styles.sectionTitle}>Animation</div>
            <div className={styles.animationSelector}>
              {availableAnimations.map((animation) => (
                <button
                  key={animation}
                  type="button"
                  className={`${styles.animationButton} ${
                    currentAnimation === animation
                      ? styles.animationButtonActive
                      : ""
                  }`}
                  onClick={() => setEntityAnimation(selectedSpawn, animation)}
                >
                  {animation}
                </button>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  )
}

// Helper to extract spawn data based on type
function getSpawnData(parsedData: StageData, spawnId: SpawnId) {
  if (!parsedData.spawns) return null

  // Count spawns by type to find the right index
  const spawnCounts = { enemy: 0, object: 0, pickup: 0, destructible: 0 }

  for (const spawn of parsedData.spawns) {
    if ("Enemy" in spawn) {
      if (spawnId.type === "enemy" && spawnCounts.enemy === spawnId.index) {
        const enemy = spawn.Enemy
        return {
          typeName: enemy.enemy_type,
          coordinates: enemy.coordinates,
          depth: enemy.depth,
          elapsed: enemy.elapsed,
          speed: enemy.speed,
        }
      }
      spawnCounts.enemy++
    } else if ("Object" in spawn) {
      if (spawnId.type === "object" && spawnCounts.object === spawnId.index) {
        const object = spawn.Object
        return {
          typeName: object.object_type,
          coordinates: object.coordinates,
          depth: object.depth,
        }
      }
      spawnCounts.object++
    } else if ("Pickup" in spawn) {
      if (spawnId.type === "pickup" && spawnCounts.pickup === spawnId.index) {
        const pickup = spawn.Pickup
        return {
          typeName: pickup.pickup_type,
          coordinates: pickup.coordinates,
          depth: pickup.depth,
          elapsed: pickup.elapsed,
        }
      }
      spawnCounts.pickup++
    } else if ("Destructible" in spawn) {
      if (
        spawnId.type === "destructible" &&
        spawnCounts.destructible === spawnId.index
      ) {
        const destructible = spawn.Destructible
        return {
          typeName: destructible.destructible_type,
          coordinates: destructible.coordinates,
          depth: destructible.depth,
          health: destructible.health,
        }
      }
      spawnCounts.destructible++
    }
  }

  return null
}

// Helper to get available animations for a spawn type
function getAvailableAnimations(spawnId: SpawnId): string[] {
  if (spawnId.type === "enemy") {
    // Return common enemy animations that most enemies support
    return ["idle", "walk", "fly", "death"]
  }

  if (spawnId.type === "destructible") {
    // Destructibles have base and broken states
    return ["base", "broken"]
  }

  // Objects and pickups don't have animations
  return []
}
