import { Application, Assets, Container, Graphics, Sprite } from "pixi.js"
import { useEffect, useRef, useState } from "react"
import { useEditorStore } from "../../state/store"
import "./Viewport.css"

const VIEWPORT_WIDTH = 800
const VIEWPORT_HEIGHT = 600
const GRID_SIZE = 32
const GRID_COLOR = 0x333333
const GRID_ALPHA = 0.3

export function Viewport() {
  const { parsedData } = useEditorStore()
  const canvasRef = useRef<HTMLDivElement>(null)
  const appRef = useRef<Application | null>(null)
  const cameraRef = useRef<Container | null>(null)
  const backgroundRef = useRef<Sprite | null>(null)
  const skyboxRef = useRef<Sprite | null>(null)
  const [cameraPos, setCameraPos] = useState({ x: 0, y: 0 })
  const [cameraScale, setCameraScale] = useState(1)
  const [isPanning, setIsPanning] = useState(false)
  const lastPanPos = useRef({ x: 0, y: 0 })

  // Initialize PixiJS application
  useEffect(() => {
    if (!canvasRef.current || !parsedData) return

    const app = new Application()
    appRef.current = app

    app
      .init({
        width: VIEWPORT_WIDTH,
        height: VIEWPORT_HEIGHT,
        backgroundColor: 0x1a1a1a,
        antialias: true,
      })
      .then(() => {
        if (canvasRef.current && app.canvas) {
          canvasRef.current.appendChild(app.canvas)

          // Create camera container
          const camera = new Container()
          cameraRef.current = camera
          app.stage.addChild(camera)
          camera.sortableChildren = true

          // Draw grid
          const grid = new Graphics()
          grid.setStrokeStyle({
            width: 1,
            color: GRID_COLOR,
            alpha: GRID_ALPHA,
          })

          // Vertical lines
          for (let x = 0; x <= VIEWPORT_WIDTH; x += GRID_SIZE) {
            grid.moveTo(x, 0)
            grid.lineTo(x, VIEWPORT_HEIGHT)
          }

          // Horizontal lines
          for (let y = 0; y <= VIEWPORT_HEIGHT; y += GRID_SIZE) {
            grid.moveTo(0, y)
            grid.lineTo(VIEWPORT_WIDTH, y)
          }

          grid.stroke()

          camera.addChild(grid)

          const loadTexture = async (path: string) => {
            const resolvedPath = path.startsWith("/") ? path : `/${path}`
            try {
              const texture = await Assets.load(resolvedPath)
              return new Sprite(texture)
            } catch (error) {
              console.error("Failed to load texture", resolvedPath, error)
              return null
            }
          }

          if (parsedData.background_path) {
            loadTexture(parsedData.background_path).then((backgroundSprite) => {
              if (!backgroundSprite) return
              backgroundSprite.position.set(0, 0)
              backgroundSprite.zIndex = -100
              camera.addChildAt(backgroundSprite, 0)
              backgroundRef.current = backgroundSprite
            })
          }

          if (parsedData.skybox?.path) {
            loadTexture(parsedData.skybox.path).then((skyboxSprite) => {
              if (!skyboxSprite) return
              skyboxSprite.position.set(0, 0)
              skyboxSprite.zIndex = -90
              camera.addChildAt(skyboxSprite, 0)
              skyboxRef.current = skyboxSprite
            })
          }
        }
      })

    return () => {
      app.destroy(true, { children: true, texture: true })
    }
  }, [parsedData])

  // Update camera position and scale
  useEffect(() => {
    if (cameraRef.current) {
      cameraRef.current.x = cameraPos.x
      cameraRef.current.y = cameraPos.y
      cameraRef.current.scale.set(cameraScale, cameraScale)
    }
  }, [cameraPos, cameraScale])

  // Handle mouse wheel zoom with native event listener (to prevent passive listener warning)
  useEffect(() => {
    const canvas = canvasRef.current
    if (!canvas) return

    const handleWheel = (event: WheelEvent) => {
      event.preventDefault()
      const zoomFactor = event.deltaY > 0 ? 0.9 : 1.1
      setCameraScale((prevScale) =>
        Math.max(0.1, Math.min(5, prevScale * zoomFactor)),
      )
    }

    canvas.addEventListener("wheel", handleWheel, { passive: false })
    return () => {
      canvas.removeEventListener("wheel", handleWheel)
    }
  }, [])

  // Handle panning
  const handlePointerDown = (event: React.PointerEvent) => {
    if (event.button === 0 || event.button === 1) {
      setIsPanning(true)
      lastPanPos.current = { x: event.clientX, y: event.clientY }
    }
  }

  const handlePointerMove = (event: React.PointerEvent) => {
    if (isPanning) {
      const dx = event.clientX - lastPanPos.current.x
      const dy = event.clientY - lastPanPos.current.y
      setCameraPos((prev) => ({
        x: prev.x + dx,
        y: prev.y + dy,
      }))
      lastPanPos.current = { x: event.clientX, y: event.clientY }
    }
  }

  const handlePointerUp = () => {
    setIsPanning(false)
  }

  if (!parsedData) {
    return (
      <div className="viewport panel">
        <div className="panel-header">Viewport</div>
        <div className="viewport-placeholder">
          <p>Load a stage file to begin editing</p>
        </div>
      </div>
    )
  }

  return (
    <div className="viewport panel">
      <div className="panel-header">
        Viewport - {parsedData.name} (Zoom: {Math.round(cameraScale * 100)}%)
      </div>
      <div
        className="viewport-canvas"
        ref={canvasRef}
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
        onPointerLeave={handlePointerUp}
      />
    </div>
  )
}
