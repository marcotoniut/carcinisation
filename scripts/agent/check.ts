#!/usr/bin/env node
import path from "node:path"
import {
  createFocusFile,
  ensureReportsDir,
  printSummary,
  REPORTS_DIR,
  runCommand,
} from "./utils"

type TaskConfig = {
  flag: string
  name: string
  command: string
  logFile: string
  focusFile: string
  matchers: RegExp[]
  phase: "parallel" | "post"
}

type TaskResult = {
  task: TaskConfig
  exitCode: number
  logFilePath: string
  focusFilePath?: string
}

const TASKS: TaskConfig[] = [
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

const aliases = new Map<string, string>()

const main = async () => {
  const args = process.argv.slice(2)
  const showInstructions = !args.includes("--instructionless")
  const requestedFlags = new Set(
    args
      .filter((arg) => arg.startsWith("--"))
      .map((arg) => aliases.get(arg) ?? arg),
  )

  const runAll = requestedFlags.has("--all")
  const selected = TASKS.filter(
    (task) => runAll || requestedFlags.has(task.flag),
  )

  if (selected.length === 0) {
    printSummary([
      "check: FAIL (no checks selected)",
      "",
      "Usage:",
      "  pnpm check:agent --lint --test --lint-web --fmt",
      "  pnpm check:agent --all",
      "",
      "Checks:",
      "  --lint       make lint",
      "  --test       make test",
      "  --lint-web   pnpm lint",
      "  --fmt        cargo fmt --all -- --check",
      "",
      "Aliases:",
      "  (no aliases)",
    ])
    process.exit(1)
  }

  await ensureReportsDir()

  const parallelTasks = selected.filter((task) => task.phase === "parallel")
  const postTasks = selected.filter((task) => task.phase === "post")

  const runTask = async (task: TaskConfig): Promise<TaskResult> => {
    const { exitCode, logFilePath } = await runCommand(
      task.command,
      task.logFile,
    )
    if (exitCode === 0) {
      return { task, exitCode, logFilePath }
    }

    const focusFilePath = path.join(REPORTS_DIR, task.focusFile)
    await createFocusFile({
      logFilePath,
      focusFilePath,
      matchers: task.matchers,
    })

    return { task, exitCode, logFilePath, focusFilePath }
  }

  const results = [
    ...(await Promise.all(parallelTasks.map(runTask))),
  ] as TaskResult[]

  for (const task of postTasks) {
    results.push(await runTask(task))
  }

  const failed = results.filter((result) => result.exitCode !== 0)
  const summaryLines: string[] = []

  for (const result of results) {
    if (result.exitCode === 0) {
      summaryLines.push(`${result.task.name}: PASS -> ${result.logFilePath}`)
    } else {
      summaryLines.push(
        `${result.task.name}: FAIL`,
        `-> focus: ${result.focusFilePath}`,
        `-> full:  ${result.logFilePath}`,
      )
    }
  }

  if (failed.length > 0 && showInstructions) {
    summaryLines.push(
      "",
      "AGENT INSTRUCTIONS:",
      "1) Open each focus file first.",
      "2) If unclear, open the full log.",
      "3) Fix the issues.",
      "4) Re-run pnpm check:agent with the same flags.",
      "5) If PASS, do not open any logs.",
    )
  }

  printSummary(summaryLines)
  process.exit(failed.length > 0 ? 1 : 0)
}

main().catch((error) => {
  printSummary([`check: FAIL (runner error) -> ${error.message}`])
  process.exit(1)
})
