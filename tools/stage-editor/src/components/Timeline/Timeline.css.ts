import { style } from "@vanilla-extract/css"
import { tokens } from "@/theme/tokens.css"

export const timeline = style({
  bottom: 0,
  left: 0,
  position: "absolute",
  right: 0,
})

export const timelineHeader = style({
  display: "flex",
  justifyContent: "space-between",
  alignItems: "center",
  marginBottom: tokens.space.sm,
})

export const timelineContent = style({
  padding: tokens.space.md,
})

export const timelineControls = style({
  display: "flex",
  flexDirection: "column",
  gap: tokens.space.xs,
})

export const timelineLabel = style({
  display: "flex",
  flexDirection: "column",
  fontSize: tokens.font.size.sm,
  fontWeight: tokens.font.weight.medium,
  gap: tokens.space.sm,
})

export const timelineSliderContainer = style({
  position: "relative",
  width: "100%",
  height: "20px",
  display: "flex",
  alignItems: "center",
})

export const sliderRoot = style({
  position: "relative",
  display: "flex",
  alignItems: "center",
  userSelect: "none",
  touchAction: "none",
  width: "100%",
  height: "20px",
})

export const sliderTrack = style({
  backgroundColor: tokens.color.border,
  position: "relative",
  flexGrow: 1,
  borderRadius: "9999px",
  height: "4px",
  overflow: "visible",
})

export const sliderRange = style({
  position: "absolute",
  backgroundColor: tokens.color.primary,
  borderRadius: "9999px",
  height: "100%",
})

export const sliderThumb = style({
  display: "block",
  width: "16px",
  height: "16px",
  backgroundColor: tokens.color.primary,
  boxShadow: tokens.shadow.sm,
  borderRadius: "50%",
  cursor: "pointer",
  transition: `all ${tokens.motion.fast}`,

  ":hover": {
    backgroundColor: tokens.color.accent,
    transform: "scale(1.2)",
  },

  ":focus": {
    outline: "none",
    boxShadow: tokens.shadow.md,
  },
})

export const timelineMarkers = style({
  position: "absolute",
  top: "50%",
  left: 0,
  width: "100%",
  height: "100%",
  pointerEvents: "none",
  transform: "translateY(-50%)",
})

export const timelineMarker = style({
  position: "absolute",
  width: "18px",
  height: "18px",
  borderRadius: "50%",
  transform: "translateX(-50%) translateY(-50%)",
  top: "50%",
  pointerEvents: "all",
  cursor: "pointer",
  transition: `all ${tokens.motion.fast}`,
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  border: "none",
  padding: 0,
  background: "transparent",

  ":hover": {
    transform: "translateX(-50%) translateY(-50%) scale(1.2)",
  },
})

export const timelineMarkerStop = style({
  backgroundColor: tokens.color.error,
  border: `2px solid ${tokens.color.error}`,
  boxShadow: `0 0 ${tokens.space.sm} ${tokens.color.error}`,

  ":hover": {
    boxShadow: `0 0 ${tokens.space.md} ${tokens.color.error}`,
  },
})

export const timelineMarkerPassed = style({
  "::after": {
    content: '""',
    position: "absolute",
    width: "8px",
    height: "8px",
    borderRadius: "50%",
    backgroundColor: tokens.color.info,
    top: "50%",
    left: "50%",
    transform: "translate(-50%, -50%)",
  },
})
