import { style } from "@vanilla-extract/css"

export const editorRoot = style({
  display: "flex",
  flexDirection: "column",
  height: "100vh",
  width: "100vw",
})

export const editorMain = style({
  display: "flex",
  flex: 1,
  overflow: "hidden",
})

export const editorCenter = style({
  display: "flex",
  flexDirection: "column",
  flex: 1,
  overflow: "hidden",
  position: "relative",
})

export const editorRightSidebar = style({
  display: "flex",
  flexDirection: "column",
  minWidth: 0,
  flexShrink: 0,
})

export const editorBottom = style({
  display: "flex",
  borderTop: "1px solid var(--color-border)",
  flexShrink: 0,
  overflow: "hidden",
})
