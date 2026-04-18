import type { FocusStyle } from "./utils.js"

export type ValidationSurface = "rust" | "web"
export type ValidationProfile = "quick" | "full" | "advisory"

export const VALIDATION_SURFACES = ["rust", "web"] as const
export const VALIDATION_PROFILES = ["quick", "full", "advisory"] as const

export type TaskConfig = {
  flag: string
  name: string
  command: string
  logFile: string
  focusFile: string
  focusStyle: FocusStyle
  matchers: RegExp[]
  phase: "parallel" | "post"
}

export const TASKS: TaskConfig[] = [
  {
    flag: "--lint",
    name: "lint-rs",
    command: "make lint",
    logFile: "lint-rs.log",
    focusFile: "lint-rs.focus.txt",
    focusStyle: "rust-or-diff",
    matchers: [/error/i, /warning/i, /clippy/i, /lint/i],
    phase: "parallel",
  },
  {
    flag: "--test",
    name: "test",
    command: "make test",
    logFile: "test.log",
    focusFile: "test.focus.txt",
    focusStyle: "cargo-test",
    matchers: [/fail/i, /FAILED/i, /panic/i, /error/i, /assert/i],
    phase: "parallel",
  },
  {
    flag: "--clippy-pedantic",
    name: "clippy-pedantic",
    command: "make clippy-pedantic",
    logFile: "clippy-pedantic.log",
    focusFile: "clippy-pedantic.focus.txt",
    focusStyle: "rustc",
    matchers: [/error/i, /warning/i, /clippy/i, /lint/i, /pedantic/i],
    phase: "parallel",
  },
  {
    flag: "--lint-biome",
    name: "lint-biome",
    command: "pnpm lint",
    logFile: "lint-biome.log",
    focusFile: "lint-biome.focus.txt",
    focusStyle: "biome",
    matchers: [/error/i, /warning/i, /biome/i, /lint/i],
    phase: "parallel",
  },
  {
    flag: "--fmt",
    name: "fmt-rs",
    command: "cargo fmt --all -- --check",
    logFile: "fmt-rs.log",
    focusFile: "fmt-rs.focus.txt",
    focusStyle: "diff",
    matchers: [/Diff in/i, /error/i, /warning/i],
    phase: "post",
  },
]

export const DEFAULT_FLAGS = ["--lint", "--test", "--fmt"] as const

export const SURFACE_FLAGS: Record<ValidationSurface, string[]> = {
  rust: ["--lint", "--test", "--fmt", "--clippy-pedantic"],
  web: ["--lint-biome"],
}

export const SURFACE_PROFILES: Record<
  ValidationSurface,
  {
    quick: string[]
    full: string[]
    advisory: string[]
  }
> = {
  rust: {
    quick: ["--lint", "--fmt"],
    full: ["--lint", "--test", "--fmt"],
    advisory: ["--clippy-pedantic"],
  },
  web: {
    quick: ["--lint-biome"],
    full: ["--lint-biome"],
    advisory: [],
  },
}

export const RULES = [
  "open_focus_file_first_on_failure",
  "rerun_same_flags_after_fix",
] as const
