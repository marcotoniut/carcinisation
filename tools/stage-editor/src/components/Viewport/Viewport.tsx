import type { FederatedPointerEvent } from "pixi.js"
import {
  Application,
  Assets,
  Container,
  Graphics,
  Point,
  Sprite,
  Text,
} from "pixi.js"
import { useCallback, useEffect, useMemo, useRef, useState } from "react"
import type { SpawnId } from "../../state/store"
import { useEditorStore } from "../../state/store"
import { getCachedTexture } from "../../utils/pixiCache"
import type { SpawnSpriteData } from "../../utils/renderSpawns"
import { renderSpawns, updateSpawnOpacity } from "../../utils/renderSpawns"
import { isSpriteAlphaHit } from "../../utils/spriteAlphaHit"
import { getCameraPosition, getStepMarkers } from "../../utils/stageTimeline"
import { calculateDistance, distanceSquaredPoints } from "./math"
import * as styles from "./Viewport.css"
import type { ScreenPoint, WorldPoint } from "./world"
import { makeWorldToScreen } from "./world"

const AXIS_COLOR = 0xbbbbbb

const LABEL_TEXT_COLOR = 0xffffff
const LABEL_TEXT_SIZE = 16
const LABEL_GAP = -5

const CAMERA_COLOR = 0x00ff00

const GRID_SIZE = 36
const GRID_EXTENT = 5000 // Grid spans -5000 to +5000 in both directions
const GRID_COLOR = 0x333333
const GRID_ALPHA = 0.3
const GAME_BOY_SCREEN_WIDTH = 160
const GAME_BOY_SCREEN_HEIGHT = 144

/// Small gap between skybox and background
const SKYBOX_TO_BACKGROUND_GAP = 20

export function Viewport() {
  const parsedData = useEditorStore((state) => state.parsedData)
  const timelinePosition = useEditorStore((state) => state.timelinePosition)
  const debugMode = useEditorStore((state) => state.debugMode)
  const entityAnimations = useEditorStore((state) => state.entityAnimations)
  const selectSpawn = useEditorStore((state) => state.selectSpawn)
  const selectedSpawn = useEditorStore((state) => state.selectedSpawn)
  const canvasRef = useRef<HTMLDivElement>(null)
  const appRef = useRef<Application | null>(null)
  const cameraRef = useRef<Container | null>(null)
  const backgroundRef = useRef<Sprite | null>(null)
  const skyboxRef = useRef<Sprite | null>(null)
  const cameraViewportRef = useRef<Graphics | null>(null)
  const debugGraphicsRef = useRef<Container | null>(null)
  const spawnContainerRef = useRef<Container | null>(null)
  const selectionRectRef = useRef<Graphics | null>(null)
  const spawnSpritesRef = useRef<SpawnSpriteData[]>([])
  const spriteLookupRef = useRef<Map<string, Sprite>>(new Map())
  const pendingSelectSpawnRef = useRef<SpawnId | null>(null)
  const pointerDownPosRef = useRef<Point | null>(null)
  const pointerIsDownRef = useRef(false)
  const [worldOriginYOffset, setWorldOriginYOffset] = useState(0)
  const [viewportSize, setViewportSize] = useState({ width: 800, height: 600 })
  // Camera position represents the world coordinates at the viewport center
  const [cameraPos, setCameraPos] = useState<WorldPoint>({ x: 0, y: 0 })
  const [cameraScale, setCameraScale] = useState(1)
  const [_isPanning, setIsPanning] = useState(false)
  const lastPanPos = useRef({ x: 0, y: 0 })
  const pinchRef = useRef<{ initialDist: number; initialScale: number } | null>(
    null,
  )
  const cameraStateRef = useRef<{
    pos: WorldPoint
    scale: number
  }>({
    pos: { x: 0, y: 0 },
    scale: 1,
  })
  const [viewportReady, setViewportReady] = useState(false)
  const isPanningRef = useRef(false)

  const worldToScreen = useMemo(
    () => makeWorldToScreen(worldOriginYOffset),
    [worldOriginYOffset],
  )

  const makeSpawnKey = useCallback(
    (spawnId: SpawnId) => `${spawnId.type}:${spawnId.index}`,
    [],
  )

  const updateSelectionRect = useCallback(() => {
    const rect = selectionRectRef.current
    if (!rect) return

    if (!selectedSpawn) {
      rect.clear()
      rect.visible = false
      return
    }

    const sprite = spriteLookupRef.current.get(makeSpawnKey(selectedSpawn))
    if (!sprite) {
      rect.clear()
      rect.visible = false
      return
    }

    const width = sprite.width
    const height = sprite.height
    const anchorX = sprite.anchor?.x ?? 0
    const anchorY = sprite.anchor?.y ?? 0
    const x = sprite.x - width * anchorX
    const y = sprite.y - height * anchorY

    rect.clear()
    rect.setStrokeStyle({ width: 1, color: 0x3399ff, alpha: 1 })
    rect.rect(x, y, width, height)
    rect.stroke()
    rect.visible = true
    rect.zIndex = (sprite.zIndex ?? 0) + 1
  }, [makeSpawnKey, selectedSpawn])

  useEffect(() => {
    updateSelectionRect()
  }, [updateSelectionRect])

  const screenToWorld = useCallback(
    (screen: ScreenPoint, camera: WorldPoint, scale: number): WorldPoint => {
      const centerX = viewportSize.width / 2
      const centerY = viewportSize.height / 2
      const cameraScreen = worldToScreen(camera)
      const worldScreenX = (screen.x - centerX + cameraScreen.x * scale) / scale
      const worldScreenY = (screen.y - centerY + cameraScreen.y * scale) / scale

      return {
        x: worldScreenX,
        y: worldOriginYOffset - worldScreenY,
      }
    },
    [
      viewportSize.width,
      viewportSize.height,
      worldOriginYOffset,
      worldToScreen,
    ],
  )

  // Calculate step markers from stage data
  const stepMarkers = useMemo(() => getStepMarkers(parsedData), [parsedData])

  const positionCameraViewport = useCallback(
    (targetTimelinePosition: number) => {
      if (!cameraViewportRef.current || !parsedData) {
        return
      }
      const cameraWorld = getCameraPosition(
        parsedData,
        targetTimelinePosition,
        stepMarkers,
      )
      const cameraScreen = worldToScreen(cameraWorld)
      cameraViewportRef.current.position.set(cameraScreen.x, cameraScreen.y)
    },
    [parsedData, stepMarkers, worldToScreen],
  )

  useEffect(() => {
    if (viewportReady) {
      positionCameraViewport(timelinePosition)
    }
  }, [viewportReady, positionCameraViewport, timelinePosition])

  // Track viewport size dynamically
  useEffect(() => {
    const container = canvasRef.current
    if (!container) return

    const updateSize = () => {
      const rect = container.getBoundingClientRect()
      setViewportSize({ width: rect.width, height: rect.height })
    }

    updateSize()

    const resizeObserver = new ResizeObserver(updateSize)
    resizeObserver.observe(container)

    return () => {
      resizeObserver.disconnect()
    }
  }, [])

  // Initialize PixiJS application
  useEffect(() => {
    setViewportReady(false)
    setWorldOriginYOffset(0)
    if (!canvasRef.current || !parsedData) return

    const app = new Application()
    appRef.current = app

    let destroyed = false
    let unmounted = false

    const destroyApp = () => {
      if (destroyed) return
      destroyed = true
      app.ticker?.stop()
      // Manually destroy display objects without touching shared Assets-managed textures.
      const destroyDisplayObject = (
        obj: Container | Graphics | Sprite | null,
      ) => {
        if (!obj) return
        if (typeof obj.destroy === "function") {
          obj.destroy({ children: true, texture: false })
        }
      }
      destroyDisplayObject(cameraRef.current)
      destroyDisplayObject(debugGraphicsRef.current)
      destroyDisplayObject(spawnContainerRef.current)
      destroyDisplayObject(selectionRectRef.current)
      destroyDisplayObject(backgroundRef.current)
      destroyDisplayObject(skyboxRef.current)
      cameraRef.current = null
      spawnContainerRef.current = null
      selectionRectRef.current = null
      backgroundRef.current = null
      skyboxRef.current = null
      // Finally destroy the app without touching textures/base textures.
      app.destroy(true, { children: false })
    }

    const initPromise = app
      .init({
        backgroundColor: 0x1a1a1a,
        antialias: true,
        resizeTo: canvasRef.current,
      })
      .then(async () => {
        if (unmounted) {
          destroyApp()
          return
        }

        if (canvasRef.current && app.canvas) {
          canvasRef.current.appendChild(app.canvas)
          app.canvas.style.imageRendering = "pixelated"

          // Create camera container
          const camera = new Container()
          cameraRef.current = camera
          app.stage.addChild(camera)
          camera.sortableChildren = true
          let stageOriginYOffset = 0

          // Draw large static grid centred at origin
          const grid = new Graphics()
          grid.setStrokeStyle({
            width: 1,
            color: GRID_COLOR,
            alpha: GRID_ALPHA,
          })

          // Vertical lines from -GRID_EXTENT to +GRID_EXTENT
          for (let x = -GRID_EXTENT; x <= GRID_EXTENT; x += GRID_SIZE) {
            grid.moveTo(x, -GRID_EXTENT)
            grid.lineTo(x, GRID_EXTENT)
          }

          // Horizontal lines from -GRID_EXTENT to +GRID_EXTENT
          for (let y = -GRID_EXTENT; y <= GRID_EXTENT; y += GRID_SIZE) {
            grid.moveTo(-GRID_EXTENT, y)
            grid.lineTo(GRID_EXTENT, y)
          }

          grid.stroke()
          grid.zIndex = -1000 // Behind everything

          camera.addChild(grid)

          const loadTexture = async (path: string) => {
            const resolvedPath = path.startsWith("/") ? path : `/${path}`
            const cached = getCachedTexture(resolvedPath)
            if (cached) return new Sprite(cached)
            try {
              const texture = await Assets.load(resolvedPath)
              return new Sprite(texture)
            } catch (error) {
              console.error("Failed to load texture", resolvedPath, error)
              return null
            }
          }

          // NOTE: All stage/timeline positions (RON/world coordinates) should be
          // converted via worldToScreen(...) before assigning to Pixi objects.
          // World is Y-up; Pixi is Y-down.

          // Load background with left edge at x=0, vertically centered
          if (parsedData.background_path) {
            const backgroundSprite = await loadTexture(
              parsedData.background_path,
            )
            if (backgroundSprite) {
              backgroundSprite.position.set(0, 0)
              backgroundSprite.zIndex = -100
              camera.addChild(backgroundSprite)
              stageOriginYOffset = backgroundSprite.height
              if (!unmounted) {
                setWorldOriginYOffset(stageOriginYOffset)
              }
              backgroundRef.current = backgroundSprite

              // Add "Background" label above the background
              const bgLabel = new Text({
                text: "Background",
                style: {
                  fontSize: LABEL_TEXT_SIZE,
                  fill: LABEL_TEXT_COLOR,
                  fontFamily: "monospace",
                },
                position: { x: 0, y: LABEL_GAP },
                anchor: { x: 0, y: 1 },
                zIndex: 100,
              })
              camera.addChild(bgLabel)
            }
          }

          // Load skybox to the LEFT of the background
          if (parsedData.skybox?.path) {
            const skyboxSprite = await loadTexture(parsedData.skybox.path)
            if (skyboxSprite && backgroundRef.current) {
              const skyboxWidth = skyboxSprite.width
              // Position skybox with a small gap to the left of the background
              skyboxSprite.position.set(
                -skyboxWidth - SKYBOX_TO_BACKGROUND_GAP,
                0,
              )
              skyboxSprite.zIndex = -90
              camera.addChild(skyboxSprite)
              skyboxRef.current = skyboxSprite

              // Add "Skybox" label above the skybox
              const skyboxLabel = new Text({
                text: "Skybox",
                style: {
                  fontSize: LABEL_TEXT_SIZE,
                  fill: LABEL_TEXT_COLOR,
                  fontFamily: "monospace",
                },
                position: {
                  x: -skyboxWidth - SKYBOX_TO_BACKGROUND_GAP,
                  y: LABEL_GAP,
                },
                anchor: { x: 0, y: 1 },
                zIndex: 100,
              })
              camera.addChild(skyboxLabel)
            }
          }

          // Create camera viewport rectangle (GameBoy screen size: 160x144)
          // Anchor at bottom-left so world origin maps to camera's corner
          const cameraViewport = new Graphics()
          cameraViewport.setStrokeStyle({ width: 1, color: CAMERA_COLOR })
          cameraViewport.rect(
            0,
            -GAME_BOY_SCREEN_HEIGHT,
            GAME_BOY_SCREEN_WIDTH,
            GAME_BOY_SCREEN_HEIGHT,
          )
          cameraViewport.stroke()
          cameraViewport.zIndex = 200
          camera.addChild(cameraViewport)
          cameraViewportRef.current = cameraViewport
          if (!unmounted) {
            setViewportReady(true)
          }

          // Create debug graphics container
          const debugContainer = new Container()
          debugContainer.zIndex = 300
          debugContainer.visible = true // Visible by default (debugMode is true)
          camera.addChild(debugContainer)
          debugGraphicsRef.current = debugContainer
          const stageWorldToScreen = makeWorldToScreen(stageOriginYOffset)

          // Draw X-axis arrow (pointing right, positive direction)
          const xAxis = new Graphics()
          xAxis.setStrokeStyle({ width: 1, color: AXIS_COLOR })
          const originScreen = stageWorldToScreen({ x: 0, y: 0 })
          const xAxisTip = stageWorldToScreen({ x: 50, y: 0 })
          xAxis.moveTo(originScreen.x, originScreen.y)
          xAxis.lineTo(xAxisTip.x, xAxisTip.y)
          // Arrow head
          const xArrowTop = stageWorldToScreen({ x: 45, y: 3 })
          const xArrowBottom = stageWorldToScreen({ x: 45, y: -3 })
          xAxis.moveTo(xAxisTip.x, xAxisTip.y)
          xAxis.lineTo(xArrowTop.x, xArrowTop.y)
          xAxis.moveTo(xAxisTip.x, xAxisTip.y)
          xAxis.lineTo(xArrowBottom.x, xArrowBottom.y)
          xAxis.stroke()
          debugContainer.addChild(xAxis)

          // Draw Y-axis arrow (pointing up, positive direction)
          const yAxis = new Graphics()
          yAxis.setStrokeStyle({ width: 1, color: AXIS_COLOR })
          const yAxisTip = stageWorldToScreen({ x: 0, y: 50 })
          yAxis.moveTo(originScreen.x, originScreen.y)
          yAxis.lineTo(yAxisTip.x, yAxisTip.y)
          // Arrow head
          const yArrowLeft = stageWorldToScreen({ x: -3, y: 45 })
          const yArrowRight = stageWorldToScreen({ x: 3, y: 45 })
          yAxis.moveTo(yAxisTip.x, yAxisTip.y)
          yAxis.lineTo(yArrowLeft.x, yArrowLeft.y)
          yAxis.moveTo(yAxisTip.x, yAxisTip.y)
          yAxis.lineTo(yArrowRight.x, yArrowRight.y)
          yAxis.stroke()
          debugContainer.addChild(yAxis)

          // Axes reflect world-space orientation via stageWorldToScreen.

          // Add origin label
          const originLabelPos = stageWorldToScreen({ x: 1, y: 1 })
          const originLabel = new Text({
            text: "0:0",
            style: {
              fontSize: 10,
              fill: AXIS_COLOR,
              fontFamily: "monospace",
            },
            position: originLabelPos,
            anchor: { x: 0, y: 0 },
          })
          debugContainer.addChild(originLabel)

          // Create spawn container for all entity sprites
          const spawnContainer = new Container()
          spawnContainer.zIndex = 50 // Between background (-100) and camera viewport (200)
          spawnContainer.sortableChildren = true
          // Use Pixi v8 event mode for hit testing; default cursor is fine.
          // eslint-disable-next-line pixi/no-unsafe-event-mode
          spawnContainer.eventMode = "static"
          camera.addChild(spawnContainer)
          spawnContainerRef.current = spawnContainer

          // Render all spawns
          const spawnSprites = await renderSpawns(
            parsedData,
            stageWorldToScreen,
            entityAnimations,
          )
          spawnSpritesRef.current = spawnSprites
          const spriteLookup = new Map<string, Sprite>()
          spriteLookupRef.current = spriteLookup

          // Add sprites to container and set up click handlers
          for (const { sprite, spawnId } of spawnSprites) {
            spawnContainer.addChild(sprite)

            const spawnKey = makeSpawnKey(spawnId)
            spriteLookup.set(spawnKey, sprite)

            // Pixi hit-testing should use alpha channel
            // eslint-disable-next-line pixi/no-unsafe-event-mode
            sprite.eventMode = "static"
            sprite.interactive = true

            // Click selection with alpha hit test; ignore if the user panned.
            sprite.on("pointertap", (event: FederatedPointerEvent) => {
              if (isPanningRef.current) return
              if (!isSpriteAlphaHit(sprite, event.global)) return
              selectSpawn(spawnId)
              updateSelectionRect()
            })
          }

          // Selection rectangle overlay
          const selectionRect = new Graphics()
          selectionRect.zIndex = 1000
          selectionRect.visible = false
          selectionRectRef.current = selectionRect
          spawnContainer.addChild(selectionRect)

          // Update initial opacity based on the current timeline position
          const currentTimeline = useEditorStore.getState().timelinePosition
          updateSpawnOpacity(spawnSprites, currentTimeline)
          updateSelectionRect()
        }
      })
      .catch((error) => {
        console.error("Failed to initialize Pixi application", error)
      })

    return () => {
      unmounted = true
      cameraRef.current = null
      backgroundRef.current = null
      skyboxRef.current = null
      cameraViewportRef.current = null
      debugGraphicsRef.current = null
      spawnContainerRef.current = null
      selectionRectRef.current = null
      spawnSpritesRef.current = []
      spriteLookupRef.current = new Map()
      if (appRef.current === app) {
        appRef.current = null
      }

      if (app.renderer) {
        destroyApp()
      } else {
        // keep the initialization promise alive so we can handle its rejection above
        void initPromise
      }
    }
  }, [
    parsedData,
    entityAnimations,
    makeSpawnKey,
    selectSpawn,
    updateSelectionRect,
  ])

  // Toggle debug graphics visibility
  useEffect(() => {
    if (debugGraphicsRef.current) {
      debugGraphicsRef.current.visible = debugMode
    }
  }, [debugMode])

  // Update spawn opacity based on timeline position
  useEffect(() => {
    if (spawnSpritesRef.current.length > 0) {
      updateSpawnOpacity(spawnSpritesRef.current, timelinePosition)
    }
  }, [timelinePosition])

  // Update camera position and scale
  // Camera position is in world coordinates (center of viewport)
  // Container position is where world (0,0) appears on screen
  useEffect(() => {
    if (cameraRef.current) {
      const centerX = viewportSize.width / 2
      const centerY = viewportSize.height / 2
      // Container position = viewport_center - camera_screen * scale
      const cameraScreen = worldToScreen(cameraPos)
      cameraRef.current.position.set(
        centerX - cameraScreen.x * cameraScale,
        centerY - cameraScreen.y * cameraScale,
      )
      cameraRef.current.scale.set(cameraScale, cameraScale)
    }
  }, [cameraPos, cameraScale, viewportSize, worldToScreen])

  // Handle mouse wheel zoom with native event listener (to prevent passive listener warning)
  const commitCameraState = useCallback(
    (nextPos: WorldPoint, nextScale: number) => {
      cameraStateRef.current = { pos: nextPos, scale: nextScale }
      setCameraPos(nextPos)
      setCameraScale(nextScale)
    },
    [],
  )

  useEffect(() => {
    const canvas = canvasRef.current
    if (!canvas) return

    const clampScale = (scale: number) => Math.max(0.1, Math.min(5, scale))

    const updateZoom = (
      scaleFactor: number,
      clientX: number,
      clientY: number,
    ) => {
      if (!canvas) return

      const rect = canvas.getBoundingClientRect()
      const mouseX = clientX - rect.left
      const mouseY = clientY - rect.top

      const prevScale = cameraStateRef.current.scale
      const prevPos = cameraStateRef.current.pos
      const newScale = clampScale(prevScale * scaleFactor)

      const pointerWorld = screenToWorld(
        { x: mouseX, y: mouseY },
        prevPos,
        prevScale,
      )
      const scaleRatio = prevScale / newScale
      const delta = new Point(
        prevPos.x - pointerWorld.x,
        prevPos.y - pointerWorld.y,
      )
      const nextPos: WorldPoint = {
        x: pointerWorld.x + delta.x * scaleRatio,
        y: pointerWorld.y + delta.y * scaleRatio,
      }

      commitCameraState(nextPos, newScale)
    }

    const handleWheel = (event: WheelEvent) => {
      event.preventDefault()
      const zoomFactor = event.deltaY > 0 ? 0.9 : 1.1
      updateZoom(zoomFactor, event.clientX, event.clientY)
    }

    canvas.addEventListener("wheel", handleWheel, {
      passive: false,
    })
    return () => {
      canvas.removeEventListener("wheel", handleWheel)
    }
  }, [commitCameraState, screenToWorld])

  // Handle panning
  const handlePointerDown = (event: React.PointerEvent) => {
    pointerIsDownRef.current = true
    pointerDownPosRef.current = new Point(event.clientX, event.clientY)
    pendingSelectSpawnRef.current = null
    isPanningRef.current = false
    lastPanPos.current = { x: event.clientX, y: event.clientY }
  }

  const handlePointerMove = (event: React.PointerEvent) => {
    if (
      pointerIsDownRef.current &&
      !isPanningRef.current &&
      event.buttons === 1 &&
      pointerDownPosRef.current
    ) {
      const dragFrom = pointerDownPosRef.current
      const dragTo = new Point(event.clientX, event.clientY)
      const distanceSq = distanceSquaredPoints(dragTo, dragFrom)
      const dragThresholdSq = 9 // 3px threshold
      if (distanceSq > dragThresholdSq) {
        // Start panning
        isPanningRef.current = true
        setIsPanning(true)
        lastPanPos.current = { x: event.clientX, y: event.clientY }
        pendingSelectSpawnRef.current = null
      }
    }

    if (isPanningRef.current) {
      const currentScreen = new Point(event.clientX, event.clientY)
      const prevScreen = new Point(lastPanPos.current.x, lastPanPos.current.y)
      const scale = cameraStateRef.current.scale
      // Keep the pointer anchored by translating the camera by the world
      // delta between the previous and current screen positions.
      const prevWorld = screenToWorld(
        { x: prevScreen.x, y: prevScreen.y },
        cameraStateRef.current.pos,
        scale,
      )
      const currentWorld = screenToWorld(
        { x: currentScreen.x, y: currentScreen.y },
        cameraStateRef.current.pos,
        scale,
      )
      const delta = new Point(
        prevWorld.x - currentWorld.x,
        prevWorld.y - currentWorld.y,
      )
      const nextPos: WorldPoint = {
        x: cameraStateRef.current.pos.x + delta.x,
        y: cameraStateRef.current.pos.y + delta.y,
      }
      commitCameraState(nextPos, scale)
      lastPanPos.current = currentScreen
    }
  }

  const handlePointerUp = () => {
    const _wasPanning = isPanningRef.current
    setIsPanning(false)
    isPanningRef.current = false
    pinchRef.current = null
    pointerIsDownRef.current = false
    pointerDownPosRef.current = null
    pendingSelectSpawnRef.current = null
  }

  const handleTouchStart = (event: React.TouchEvent) => {
    if (event.touches.length === 2) {
      pinchRef.current = {
        initialDist: calculateDistance(event.touches),
        initialScale: cameraScale,
      }
    }
  }

  const handleTouchMove = (event: React.TouchEvent) => {
    if (pinchRef.current && event.touches.length === 2) {
      const dist = calculateDistance(event.touches)
      if (pinchRef.current.initialDist > 0 && dist > 0) {
        const scaleFactor = dist / pinchRef.current.initialDist
        const nextScale = Math.max(
          0.1,
          Math.min(5, pinchRef.current?.initialScale * scaleFactor),
        )
        commitCameraState(cameraStateRef.current.pos, nextScale)
      }
      event.preventDefault()
    }
  }

  const handleTouchEnd = (event: React.TouchEvent) => {
    if (event.touches.length < 2) {
      pinchRef.current = null
    }
  }

  if (!parsedData) {
    return (
      <div className={`${styles.viewport} panel`}>
        <div className="panel-header">Viewport</div>
        <div className={styles.viewportPlaceholder}>
          <p>Load a stage file to begin editing</p>
        </div>
      </div>
    )
  }

  return (
    <div className={`${styles.viewport} panel`}>
      <div className="panel-header">
        Viewport - {parsedData.name} (Zoom: {Math.round(cameraScale * 100)}%)
      </div>
      <div
        className={styles.viewportCanvas}
        ref={canvasRef}
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
        onPointerLeave={handlePointerUp}
        onTouchStart={handleTouchStart}
        onTouchMove={handleTouchMove}
        onTouchEnd={handleTouchEnd}
      />
    </div>
  )
}
