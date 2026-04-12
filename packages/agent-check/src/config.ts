export type TaskConfig = {
  flag: string
  name: string
  command: string
  logFile: string
  focusFile: string
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
    matchers: [/error/i, /warning/i, /clippy/i, /lint/i],
    phase: "parallel",
  },
  {
    flag: "--test",
    name: "test",
    command: "make test",
    logFile: "test.log",
    focusFile: "test.focus.txt",
    matchers: [/fail/i, /FAILED/i, /panic/i, /error/i, /assert/i],
    phase: "parallel",
  },
  {
    flag: "--lint-web",
    name: "lint-web",
    command: "pnpm lint",
    logFile: "lint-web.log",
    focusFile: "lint-web.focus.txt",
    matchers: [/error/i, /warning/i, /biome/i, /lint/i],
    phase: "parallel",
  },
  {
    flag: "--fmt",
    name: "fmt-rs",
    command: "cargo fmt --all -- --check",
    logFile: "fmt-rs.log",
    focusFile: "fmt-rs.focus.txt",
    matchers: [/Diff in/i, /error/i, /warning/i],
    phase: "post",
  },
]

export const DEFAULT_FLAGS = ["--all"] as const

export const RULES = [
  "open_focus_file_first_on_failure",
  "rerun_same_flags_after_fix",
] as const
