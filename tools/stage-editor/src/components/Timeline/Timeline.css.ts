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

export const timelineSlider = style({
  width: "100%",
  cursor: "pointer",
  margin: 0,
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
