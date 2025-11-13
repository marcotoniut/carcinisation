import * as styles from "./ResizeHandle.css"

interface ResizeHandleProps {
  direction: "horizontal" | "vertical"
  onMouseDown: (event: React.MouseEvent) => void
  isResizing?: boolean
}

export function ResizeHandle({
  direction,
  onMouseDown,
  isResizing = false,
}: ResizeHandleProps) {
  const className = `${styles.resizeHandle} ${
    direction === "horizontal"
      ? styles.resizeHandleHorizontal
      : styles.resizeHandleVertical
  } ${isResizing ? styles.resizeHandleActive : ""}`

  return (
    <button
      type="button"
      aria-label={`Resize ${direction === "horizontal" ? "horizontally" : "vertically"}`}
      className={className}
      onMouseDown={onMouseDown}
    />
  )
}
