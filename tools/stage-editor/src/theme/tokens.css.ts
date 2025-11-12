/**
 * Design tokens for the stage editor
 */
export const tokens = {
  color: {
    bg: "#0a0a0a",
    text: "#e5e5e5",
    textMuted: "#a1a1aa",
    border: "#333333",
    surface: "#1a1a1a",
    surfaceAlt: "#262626",

    primary: "#6366f1",
    accent: "#8b5cf6",

    success: "#22c55e",
    error: "#ef4444",
    warning: "#f59e0b",
    info: "#3b82f6",

    hover: "rgba(255, 255, 255, 0.1)",
    focus: "rgba(99, 102, 241, 0.3)",
  },
  radius: {
    sm: "4px",
    md: "6px",
    lg: "8px",
  },
  space: {
    xs: "4px",
    sm: "8px",
    md: "12px",
    lg: "16px",
    xl: "24px",
  },
  shadow: {
    sm: "0 1px 2px rgba(0, 0, 0, 0.3)",
    md: "0 4px 6px rgba(0, 0, 0, 0.4)",
    lg: "0 10px 15px rgba(0, 0, 0, 0.5)",
  },
  z: {
    base: "0",
    panel: "10",
    overlay: "100",
    toast: "9999",
  },
  motion: {
    fast: "120ms",
    normal: "200ms",
  },
  font: {
    family: "system-ui, -apple-system, sans-serif",
    size: {
      xs: "11px",
      sm: "12px",
      md: "14px",
      lg: "16px",
    },
    weight: {
      regular: "400",
      medium: "500",
      semibold: "600",
    },
  },
} as const
