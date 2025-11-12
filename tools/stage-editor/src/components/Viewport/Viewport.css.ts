import { style } from "@vanilla-extract/css"
import { tokens } from "@/theme/tokens.css"

export const viewport = style({
  flex: 1,
  display: "flex",
  flexDirection: "column",
})

export const viewportCanvas = style({
  flex: 1,
  background: tokens.color.bg,
  cursor: "crosshair",
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  overflow: "hidden",
  touchAction: "none",
})

export const viewportPlaceholder = style({
  flex: 1,
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  color: tokens.color.textMuted,
  fontSize: "14px",
})
