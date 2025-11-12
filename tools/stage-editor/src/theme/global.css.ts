import { globalStyle } from "@vanilla-extract/css"
import { tokens } from "./tokens.css"

globalStyle("html, body", {
  margin: 0,
  padding: 0,
  height: "100%",
  overflow: "hidden",
  fontFamily: tokens.font.family,
  fontSize: tokens.font.size.md,
  color: tokens.color.text,
  background: tokens.color.bg,
})

globalStyle("#root", {
  height: "100%",
  display: "flex",
  flexDirection: "column",
})

globalStyle("*, *::before, *::after", {
  boxSizing: "border-box",
})
