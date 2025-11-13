import { style } from "@vanilla-extract/css"
import { tokens } from "@/theme/tokens.css"

export const toolbar = style({
  display: "flex",
  alignItems: "center",
  gap: tokens.space.md,
  padding: tokens.space.sm,
  background: tokens.color.surface,
  borderBottom: `1px solid ${tokens.color.border}`,
  height: "48px",
})

export const toolbarSection = style({
  display: "flex",
  alignItems: "center",
  gap: tokens.space.sm,
})

export const toolbarTitle = style({
  fontSize: "16px",
  fontWeight: 600,
  margin: 0,
})

export const toolbarFilename = style({
  fontSize: "14px",
  color: tokens.color.textMuted,
  marginLeft: tokens.space.md,
})

export const toolbarDirty = style({
  color: tokens.color.warning,
})

export const toolbarButtonActive = style({
  backgroundColor: tokens.color.primary,
  color: tokens.color.bg,
  fontWeight: tokens.font.weight.semibold,
})
