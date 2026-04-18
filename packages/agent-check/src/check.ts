#!/usr/bin/env node
/** Runs lint, test, and format checks in parallel, writing logs and focus files to `reports/agent/`. */

import path from "node:path"
import type { FailureType } from "./check-support.js"
import { classifyFailure } from "./check-support.js"
import type {
  TaskConfig,
  ValidationProfile,
  ValidationSurface,
} from "./config.js"
import {
  DEFAULT_FLAGS,
  RULES,
  SURFACE_FLAGS,
  SURFACE_PROFILES,
  TASKS,
} from "./config.js"
import { resolveRequestedFlags } from "./selection.js"
import {
  createFocusFile,
  ensureReportsDir,
  printSummary,
  REPORTS_DIR,
  runCommand,
} from "./utils.js"

const VERSION = 1

type TaskResult = {
  task: TaskConfig
  exitCode: number
  logFilePath: string
  focusFilePath?: string
  failureType?: FailureType
}

const aliases = new Map<string, string>()

const emitJson = (payload: unknown): void => {
  process.stdout.write(`${JSON.stringify(payload)}\n`)
}

const buildListPayload = () => ({
  version: VERSION,
  defaultFlags: [...DEFAULT_FLAGS],
  surfaceFlags: Object.fromEntries(
    Object.entries(SURFACE_FLAGS).map(([surface, flags]) => [
      surface,
      [...flags],
    ]),
  ) as Record<ValidationSurface, string[]>,
  surfaceProfiles: Object.fromEntries(
    Object.entries(SURFACE_PROFILES).map(([surface, profiles]) => [
      surface,
      {
        quick: [...profiles.quick],
        full: [...profiles.full],
        advisory: [...profiles.advisory],
      },
    ]),
  ) as Record<ValidationSurface, Record<ValidationProfile, string[]>>,
  tasks: TASKS.map((task) => ({
    flag: task.flag,
    name: task.name,
    command: task.command,
    logFile: task.logFile,
    focusFile: task.focusFile,
    phase: task.phase,
  })),
  rules: [...RULES],
})

const buildListLines = (): string[] => {
  const lines = [
    "Available checks:",
    ...TASKS.map((task) => `  ${task.flag}  ${task.command}`),
    "",
    `Default flags: ${DEFAULT_FLAGS.join(" ")}`,
    "",
    "Surfaces:",
    ...Object.entries(SURFACE_FLAGS).map(
      ([surface, flags]) => `  ${surface}: ${flags.join(" ")}`,
    ),
    "",
    "Profiles:",
  ]

  for (const [surface, profiles] of Object.entries(SURFACE_PROFILES)) {
    lines.push(`  ${surface} quick: ${profiles.quick.join(" ")}`)
    lines.push(`  ${surface} full: ${profiles.full.join(" ")}`)
    lines.push(
      `  ${surface} advisory: ${profiles.advisory.join(" ") || "(none)"}`,
    )
  }

  return lines
}

const runTask = async (task: TaskConfig): Promise<TaskResult> => {
  const { exitCode, logFilePath, stdout, stderr } = await runCommand(
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
    focusStyle: task.focusStyle,
    matchers: task.matchers,
  })

  return {
    task,
    exitCode,
    logFilePath,
    focusFilePath,
    failureType: classifyFailure(`${stdout}\n${stderr}`),
  }
}

const runTasks = async (
  selected: TaskConfig[],
  failFast: boolean,
): Promise<TaskResult[]> => {
  if (failFast) {
    const results: TaskResult[] = []

    for (const task of selected) {
      const result = await runTask(task)
      results.push(result)
      if (result.exitCode !== 0) {
        break
      }
    }

    return results
  }

  const parallelTasks = selected.filter((task) => task.phase === "parallel")
  const postTasks = selected.filter((task) => task.phase === "post")
  const results = [
    ...(await Promise.all(parallelTasks.map(runTask))),
  ] as TaskResult[]

  for (const task of postTasks) {
    results.push(await runTask(task))
  }

  return results
}

const main = async () => {
  const args = process.argv.slice(2)
  const showInstructions = !args.includes("--instructionless")
  const wantsJson = args.includes("--json")
  const wantsList = args.includes("--list")
  const failFast = args.includes("--fail-fast")

  if (wantsList) {
    if (wantsJson) {
      emitJson(buildListPayload())
    } else {
      printSummary(buildListLines())
    }
    process.exit(0)
  }

  let requestedFlags: Set<string>
  try {
    requestedFlags = resolveRequestedFlags(args, { aliases })
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error)
    if (wantsJson) {
      emitJson({
        version: VERSION,
        error: message,
        exitCode: 1,
      })
    } else {
      printSummary([`check: FAIL (runner error) -> ${message}`])
    }
    process.exit(1)
  }

  const runAll = requestedFlags.has("--all")
  const selected = TASKS.filter(
    (task) => runAll || requestedFlags.has(task.flag),
  )

  if (selected.length === 0) {
    const usageLines = [
      "check: FAIL (no checks selected)",
      "",
      "Usage:",
      "  pnpm check:agent --lint --test --clippy-pedantic --lint-biome --fmt",
      "  pnpm check:agent --surface rust --profile quick",
      "  pnpm check:agent --surface rust --surface web --profile full",
      `  pnpm check:agent ${DEFAULT_FLAGS.join(" ")}`,
      "",
      "Checks:",
      "  --lint       make lint",
      "  --test       make test",
      "  --clippy-pedantic  make clippy-pedantic",
      "  --lint-biome  pnpm lint",
      "  --fmt        cargo fmt --all -- --check",
      "",
      "Surfaces:",
      "  rust         --lint --test --fmt --clippy-pedantic",
      "  web          --lint-biome",
      "",
      "Profiles:",
      "  quick        iteration checks for selected surfaces",
      "  full         handoff/review checks for selected surfaces",
      "  advisory     opt-in quality-debt checks for selected surfaces",
    ]
    if (wantsJson) {
      emitJson({
        version: VERSION,
        error: "no checks selected",
        usage: usageLines,
        exitCode: 1,
      })
    } else {
      printSummary(usageLines)
    }
    process.exit(1)
  }

  await ensureReportsDir()
  const startedAt = new Date().toISOString()
  const results = await runTasks(selected, failFast)

  const failed = results.filter((result) => result.exitCode !== 0)
  if (wantsJson) {
    emitJson({
      version: VERSION,
      startedAt,
      requestedFlags: [...requestedFlags],
      selectedTasks: selected.map((task) => task.flag),
      failFast,
      results: results.map((result) => ({
        flag: result.task.flag,
        name: result.task.name,
        status: result.exitCode === 0 ? "pass" : "fail",
        exitCode: result.exitCode,
        logFilePath: result.logFilePath,
        focusFilePath: result.focusFilePath ?? null,
        failureType: result.failureType ?? null,
      })),
      exitCode: failed.length > 0 ? 1 : 0,
    })
    process.exit(failed.length > 0 ? 1 : 0)
  }

  const summaryLines: string[] = []

  for (const result of results) {
    if (result.exitCode === 0) {
      summaryLines.push(`${result.task.name}: PASS -> ${result.logFilePath}`)
    } else {
      summaryLines.push(
        `${result.task.name}: FAIL`,
        ...(result.failureType ? [`-> type:  ${result.failureType}`] : []),
        `-> focus: ${result.focusFilePath}`,
        `-> full:  ${result.logFilePath}`,
      )
    }
  }

  if (failed.length > 0 && showInstructions) {
    summaryLines.push(
      "",
      "AGENT INSTRUCTIONS:",
      `1) ${RULES[0].replaceAll("_", " ")}.`,
      "2) If unclear, open the full log.",
      "3) Fix the issues.",
      `4) ${RULES[1].replaceAll("_", " ")}.`,
      "5) If PASS, do not open any logs.",
    )
  }

  printSummary(summaryLines)
  process.exit(failed.length > 0 ? 1 : 0)
}

main().catch((error) => {
  const message = error instanceof Error ? error.message : String(error)
  if (process.argv.slice(2).includes("--json")) {
    emitJson({
      version: VERSION,
      error: message,
      exitCode: 1,
    })
    process.exit(1)
  }

  printSummary([`check: FAIL (runner error) -> ${message}`])
  process.exit(1)
})
