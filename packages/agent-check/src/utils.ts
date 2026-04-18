/** Shared helpers for running commands, capturing logs, and generating focus files. */

import { spawn } from "node:child_process"
import { mkdir, readFile, writeFile } from "node:fs/promises"
import path from "node:path"
import { fileURLToPath } from "node:url"

const scriptDir = path.dirname(fileURLToPath(import.meta.url))
export const REPO_ROOT = path.resolve(scriptDir, "../../..")

/** Absolute path to the agent reports directory (`reports/agent/`). */
export const REPORTS_DIR = path.resolve(REPO_ROOT, "reports", "agent")

/** Creates the reports directory if it doesn't already exist. */
export const ensureReportsDir = async (): Promise<void> => {
  await mkdir(REPORTS_DIR, { recursive: true })
}

export type FocusStyle =
  | "matched-lines"
  | "rustc"
  | "cargo-test"
  | "biome"
  | "diff"
  | "rust-or-diff"

type FocusOptions = {
  logFilePath: string
  focusFilePath: string
  focusStyle: FocusStyle
  matchers: RegExp[]
  maxLines?: number
  tailLines?: number
}

type BuildFocusOptions = {
  content: string
  focusStyle: FocusStyle
  matchers: RegExp[]
  maxLines?: number
  tailLines?: number
}

type CommandResult = {
  exitCode: number
  stdout: string
  stderr: string
  logFilePath: string
}

/** Spawns a shell command, captures stdout/stderr, and writes combined output to a log file. */
export const runCommand = async (
  command: string,
  logFileName: string,
): Promise<CommandResult> => {
  await ensureReportsDir()
  const logFilePath = path.join(REPORTS_DIR, logFileName)
  let stdout = ""
  let stderr = ""

  const child = spawn(command, {
    shell: true,
    cwd: REPO_ROOT,
    env: { ...process.env },
  })

  child.stdout.on("data", (chunk) => {
    stdout += chunk.toString()
  })
  child.stderr.on("data", (chunk) => {
    stderr += chunk.toString()
  })

  const exitCode = await new Promise<number>((resolve) => {
    child.on("close", (code) => resolve(code ?? 0))
  })

  await writeFile(logFilePath, `${stdout}${stderr}`, "utf8")

  return { exitCode, stdout, stderr, logFilePath }
}

const dedupePreservingOrder = (lines: string[]): string[] => {
  const seen = new Set<string>()
  const unique: string[] = []

  for (const line of lines) {
    if (seen.has(line)) continue
    seen.add(line)
    unique.push(line)
  }

  return unique
}

const summarizeRustDiagnostics = (lines: string[]): string[] => {
  const summaries: string[] = []

  for (let i = 0; i < lines.length; i += 1) {
    const diagnostic = lines[i]?.match(/^(error|warning):\s+(.*)$/)
    if (!diagnostic) continue

    const severity = diagnostic[1]
    const message = diagnostic[2].trim()
    let location: string | undefined
    let lintId: string | undefined
    let helpText: string | undefined

    for (
      let j = i + 1;
      j < lines.length && !/^(error|warning):\s+/.test(lines[j] ?? "");
      j += 1
    ) {
      const line = lines[j] ?? ""
      const locationMatch = line.match(/^\s*-->\s+(.+:\d+:\d+)/)
      if (locationMatch) {
        location ??= locationMatch[1]
      }

      const clippyMatch = line.match(/\bclippy::([a-z0-9-]+)/i)
      if (clippyMatch) {
        lintId ??= `clippy::${clippyMatch[1]}`
      }

      const rustcUrlMatch = line.match(/#([a-z0-9_]+)\s*$/i)
      if (rustcUrlMatch && lintId === undefined) {
        lintId = `clippy::${rustcUrlMatch[1].replaceAll("_", "-")}`
      }

      const helpMatch = line.match(/^\s*= help:\s+(.*)$/)
      if (
        helpMatch &&
        helpText === undefined &&
        !helpMatch[1].startsWith("for further information visit")
      ) {
        helpText = helpMatch[1]
      }
    }

    const parts = [`${severity}: ${message}`]
    if (location) parts.unshift(location)
    if (lintId) parts.push(`[${lintId}]`)
    if (helpText) parts.push(`help: ${helpText}`)
    summaries.push(parts.join(" | "))
  }

  return dedupePreservingOrder(summaries)
}

const summarizeBiomeDiagnostics = (lines: string[]): string[] => {
  const summaries: string[] = []
  const headerPattern = /^(\S+(?::\d+:\d+)?)\s+(.+?)\s+━+$/u

  for (let i = 0; i < lines.length; i += 1) {
    const header = lines[i]?.match(headerPattern)
    if (!header) continue

    const location = header[1]
    const category = header[2].replace(/\s+FIXABLE$/u, "").trim()
    let message = ""

    for (
      let j = i + 1;
      j < lines.length && !headerPattern.test(lines[j] ?? "");
      j += 1
    ) {
      const line = lines[j] ?? ""
      const messageMatch = line.match(/^\s*[×!]\s+(.*)$/u)
      if (messageMatch) {
        message = messageMatch[1].trim()
        break
      }
    }

    summaries.push(
      message
        ? `${location} | ${category} | ${message}`
        : `${location} | ${category}`,
    )
  }

  const finalCount = lines.find((line) => /^Found \d+ error/.test(line))
  return finalCount
    ? [...dedupePreservingOrder(summaries), finalCount]
    : dedupePreservingOrder(summaries)
}

const summarizeCargoTest = (lines: string[]): string[] => {
  const summaries: string[] = []

  for (let i = 0; i < lines.length; i += 1) {
    const header = lines[i]?.match(/^----\s+(.+?)\s+stdout\s+----$/)
    if (!header) continue

    const testName = header[1]
    let location: string | undefined
    let message: string | undefined

    for (
      let j = i + 1;
      j < lines.length && !/^----\s+.+?\s+stdout\s+----$/.test(lines[j] ?? "");
      j += 1
    ) {
      const line = lines[j] ?? ""
      const panicMatch = line.match(/^thread '.*' panicked at (.+)$/)
      if (panicMatch) {
        const detail = panicMatch[1]
        const locationMatch = detail.match(/(.+:\d+:\d+):\s*(.*)$/)
        if (locationMatch) {
          location = locationMatch[1]
          if (locationMatch[2].trim().length > 0) {
            message = locationMatch[2].trim()
          }
        } else if (detail.trim().length > 0) {
          message = detail.trim()
        }
        continue
      }

      if (
        message === undefined &&
        line.trim().length > 0 &&
        !line.startsWith("note:") &&
        !line.startsWith("stack backtrace:")
      ) {
        message = line.trim()
      }
    }

    const parts = [testName]
    if (location) parts.push(location)
    if (message) parts.push(message)
    summaries.push(parts.join(" | "))
  }

  const finalResult = lines.find((line) => /^test result: FAILED/.test(line))
  if (finalResult) summaries.push(finalResult.trim())

  return dedupePreservingOrder(summaries)
}

const summarizeDiffs = (lines: string[]): string[] =>
  dedupePreservingOrder(lines.filter((line) => /^Diff in /.test(line)))

const fallbackMatchedLines = ({
  lines,
  matchers,
  maxLines,
  tailLines,
}: {
  lines: string[]
  matchers: RegExp[]
  maxLines: number
  tailLines: number
}): string[] => {
  const matched = lines.filter((line) =>
    matchers.some((matcher) => matcher.test(line)),
  )

  if (matched.length > 0) {
    return matched.slice(0, maxLines)
  }

  return lines.slice(Math.max(lines.length - tailLines, 0))
}

export const buildFocusLines = ({
  content,
  focusStyle,
  matchers,
  maxLines = 50,
  tailLines = 50,
}: BuildFocusOptions): string[] => {
  const lines = content.split(/\r?\n/)
  let focusLines: string[] = []

  switch (focusStyle) {
    case "rustc":
      focusLines = summarizeRustDiagnostics(lines)
      break
    case "cargo-test":
      focusLines = summarizeCargoTest(lines)
      if (focusLines.length === 0) {
        focusLines = summarizeRustDiagnostics(lines)
      }
      break
    case "biome":
      focusLines = summarizeBiomeDiagnostics(lines)
      break
    case "diff":
      focusLines = summarizeDiffs(lines)
      break
    case "rust-or-diff":
      focusLines = summarizeDiffs(lines)
      if (focusLines.length === 0) {
        focusLines = summarizeRustDiagnostics(lines)
      }
      break
    case "matched-lines":
      break
  }

  if (focusLines.length === 0) {
    focusLines = fallbackMatchedLines({ lines, matchers, maxLines, tailLines })
  }

  if (focusLines.length === 0) {
    return ["(no output captured)"]
  }

  return focusLines.slice(0, maxLines)
}

/** Extracts target-specific failure lines from a log file into a shorter focus file. */
export const createFocusFile = async ({
  logFilePath,
  focusFilePath,
  focusStyle,
  matchers,
  maxLines = 50,
  tailLines = 50,
}: FocusOptions): Promise<void> => {
  let content = ""
  try {
    content = await readFile(logFilePath, "utf8")
  } catch {
    await writeFile(focusFilePath, "(no output captured)\n", "utf8")
    return
  }

  const focusLines = buildFocusLines({
    content,
    focusStyle,
    matchers,
    maxLines,
    tailLines,
  })
  await writeFile(focusFilePath, `${focusLines.join("\n")}\n`, "utf8")
}

/** Writes summary lines to stdout. */
export const printSummary = (lines: string[]): void => {
  process.stdout.write(`${lines.join("\n")}\n`)
}
