import { useCallback, useEffect, useRef, useState } from "react"

interface UseResizableOptions {
  storageKey: string
  defaultSize: number
  minSize?: number
  maxSize?: number
  direction: "horizontal" | "vertical"
}

export function useResizable({
  storageKey,
  defaultSize,
  minSize = 100,
  maxSize = Infinity,
  direction,
}: UseResizableOptions) {
  const [size, setSize] = useState(() => {
    const stored = localStorage.getItem(storageKey)
    return stored ? Number.parseFloat(stored) : defaultSize
  })

  const [isResizing, setIsResizing] = useState(false)
  const startPosRef = useRef(0)
  const startSizeRef = useRef(0)

  useEffect(() => {
    localStorage.setItem(storageKey, size.toString())
  }, [storageKey, size])

  const handleMouseDown = useCallback(
    (event: React.MouseEvent) => {
      event.preventDefault()
      setIsResizing(true)
      startPosRef.current =
        direction === "horizontal" ? event.clientX : event.clientY
      startSizeRef.current = size
    },
    [direction, size],
  )

  useEffect(() => {
    if (!isResizing) return

    const handleMouseMove = (event: MouseEvent) => {
      const currentPos =
        direction === "horizontal" ? event.clientX : event.clientY
      const delta = currentPos - startPosRef.current
      const newSize = Math.max(
        minSize,
        Math.min(maxSize, startSizeRef.current + delta),
      )
      setSize(newSize)
    }

    const handleMouseUp = () => {
      setIsResizing(false)
    }

    document.addEventListener("mousemove", handleMouseMove)
    document.addEventListener("mouseup", handleMouseUp)

    return () => {
      document.removeEventListener("mousemove", handleMouseMove)
      document.removeEventListener("mouseup", handleMouseUp)
    }
  }, [isResizing, direction, minSize, maxSize])

  return { size, isResizing, handleMouseDown }
}
