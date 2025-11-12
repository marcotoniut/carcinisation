import { style } from "@vanilla-extract/css"
import { tokens } from "@/theme/tokens.css"

export const console = style({
  height: "150px",
})

export const consoleContent = style({
  padding: tokens.space.sm,
  fontFamily: "monospace",
  fontSize: "12px",
  overflowY: "auto",
  maxHeight: "100px",
})

export const consoleMessage = style({
  padding: `${tokens.space.xs} 0`,
  borderBottom: `1px solid ${tokens.color.border}`,
})

export const consoleMessageInfo = style({
  color: tokens.color.textMuted,
})

export const consoleMessageError = style({
  color: tokens.color.error,
})

export const consoleMessageWarning = style({
  color: tokens.color.warning,
})

export const consoleMessageSuccess = style({
  color: tokens.color.success,
})
