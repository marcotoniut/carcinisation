import { style } from "@vanilla-extract/css"

export const inspector = style({
  display: "flex",
  flexDirection: "column",
  height: "100%",
  overflow: "hidden",
})

export const inspectorContent = style({
  flex: 1,
  overflowY: "auto",
  padding: "12px",
  fontSize: "13px",
})

export const inspectorEmpty = style({
  padding: "24px 12px",
  color: "#888",
  fontStyle: "italic",
  textAlign: "center",
})

export const propertyGroup = style({
  marginBottom: "16px",
})

export const propertyLabel = style({
  display: "block",
  marginBottom: "4px",
  fontSize: "11px",
  color: "#aaa",
  fontWeight: "600",
  textTransform: "uppercase",
  letterSpacing: "0.5px",
})

export const propertyValue = style({
  padding: "6px 8px",
  background: "#1a1a1a",
  border: "1px solid #333",
  borderRadius: "3px",
  fontSize: "13px",
  fontFamily: "monospace",
})

export const animationSelector = style({
  marginTop: "8px",
  display: "flex",
  flexDirection: "column",
  gap: "4px",
})

export const animationButton = style({
  padding: "6px 12px",
  background: "#2a2a2a",
  border: "1px solid #444",
  borderRadius: "3px",
  cursor: "pointer",
  fontSize: "12px",
  color: "#ddd",
  transition: "all 0.15s",
  ":hover": {
    background: "#333",
    borderColor: "#555",
  },
})

export const animationButtonActive = style({
  background: "#0066cc",
  borderColor: "#0077ee",
  color: "#fff",
  fontWeight: "600",
})

export const sectionTitle = style({
  fontSize: "12px",
  fontWeight: "600",
  color: "#ccc",
  marginBottom: "8px",
  paddingBottom: "4px",
  borderBottom: "1px solid #333",
})
