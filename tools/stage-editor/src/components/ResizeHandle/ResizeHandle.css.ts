import { style } from "@vanilla-extract/css"
import { tokens } from "@/theme/tokens.css"

export const resizeHandle = style({
  position: "relative",
  zIndex: 10,
  transition: `background-color ${tokens.motion.fast}`,
  border: "none",
  padding: 0,
  backgroundColor: "transparent",
  margin: 0,

  ":hover": {
    backgroundColor: tokens.color.primary,
  },

  ":focus": {
    outline: "none",
  },
})

export const resizeHandleHorizontal = style({
  width: "4px",
  cursor: "ew-resize",
  minWidth: "4px",
  flexShrink: 0,

  ":hover": {
    width: "6px",
  },
})

export const resizeHandleVertical = style({
  height: "4px",
  cursor: "ns-resize",
  minHeight: "4px",
  flexShrink: 0,

  ":hover": {
    height: "6px",
  },
})

export const resizeHandleActive = style({
  backgroundColor: tokens.color.primary,
})
