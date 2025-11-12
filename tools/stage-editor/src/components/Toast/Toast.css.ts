import { keyframes, style } from "@vanilla-extract/css"
import { tokens } from "@/theme/tokens.css"

const slideIn = keyframes({
  from: {
    transform: "translateX(calc(100% + 24px))",
  },
  to: {
    transform: "translateX(0)",
  },
})

const hide = keyframes({
  from: {
    opacity: 1,
  },
  to: {
    opacity: 0,
  },
})

const swipeOut = keyframes({
  from: {
    transform: "translateX(var(--radix-toast-swipe-end-x))",
  },
  to: {
    transform: "translateX(calc(100% + 24px))",
  },
})

export const toastViewport = style({
  position: "fixed",
  bottom: 0,
  right: 0,
  display: "flex",
  flexDirection: "column",
  padding: "24px",
  gap: "10px",
  width: "390px",
  maxWidth: "100vw",
  margin: 0,
  listStyle: "none",
  zIndex: tokens.z.toast,
  outline: "none",
})

export const toast = style({
  backgroundColor: tokens.color.surface,
  border: `1px solid ${tokens.color.border}`,
  borderRadius: tokens.radius.md,
  boxShadow: tokens.shadow.lg,
  padding: "16px",
  display: "flex",
  alignItems: "flex-start",
  gap: "12px",
  animation: `${slideIn} 150ms cubic-bezier(0.16, 1, 0.3, 1)`,

  selectors: {
    '&[data-state="closed"]': {
      animation: `${hide} 100ms ease-in`,
    },
    '&[data-swipe="move"]': {
      transform: "translateX(var(--radix-toast-swipe-move-x))",
    },
    '&[data-swipe="cancel"]': {
      transform: "translateX(0)",
      transition: "transform 200ms ease-out",
    },
    '&[data-swipe="end"]': {
      animation: `${swipeOut} 100ms ease-out`,
    },
  },
})

export const toastSuccess = style({
  borderLeft: `3px solid ${tokens.color.success}`,
})

export const toastError = style({
  borderLeft: `3px solid ${tokens.color.error}`,
})

export const toastInfo = style({
  borderLeft: `3px solid ${tokens.color.info}`,
})

export const toastContent = style({
  flex: 1,
  display: "flex",
  flexDirection: "column",
  gap: "4px",
})

export const toastTitle = style({
  color: tokens.color.text,
  fontSize: "14px",
  fontWeight: 500,
  lineHeight: 1.4,
  margin: 0,
})

export const toastDescription = style({
  color: tokens.color.textMuted,
  fontSize: "13px",
  lineHeight: 1.4,
  margin: 0,
})

export const toastClose = style({
  background: "none",
  border: "none",
  color: tokens.color.textMuted,
  cursor: "pointer",
  fontSize: "20px",
  lineHeight: 1,
  padding: 0,
  width: "20px",
  height: "20px",
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  borderRadius: tokens.radius.sm,
  transition: "color 0.15s, background-color 0.15s",

  ":hover": {
    color: tokens.color.text,
    backgroundColor: tokens.color.hover,
  },
})
