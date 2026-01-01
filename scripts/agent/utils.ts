import { spawn } from "node:child_process"
import { mkdir, readFile, writeFile } from "node:fs/promises"
import path from "node:path"

export const REPORTS_DIR = path.resolve(process.cwd(), "reports", "agent")

export const ensureReportsDir = async (): Promise<void> => {
  await mkdir(REPORTS_DIR, { recursive: true })
}

type FocusOptions = {
  logFilePath: string
  focusFilePath: string
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

export const createFocusFile = async ({
  logFilePath,
  focusFilePath,
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

  const lines = content.split(/\r?\n/)
  const matched: string[] = []

  for (const line of lines) {
    if (matchers.some((matcher) => matcher.test(line))) {
      matched.push(line)
    }
  }

  let focusLines =
    matched.length > 0
      ? matched.slice(0, maxLines)
      : lines.slice(Math.max(lines.length - tailLines, 0))

  if (focusLines.length === 0) {
    focusLines = ["(no output captured)"]
  }

  await writeFile(focusFilePath, `${focusLines.join("\n")}\n`, "utf8")
}

export const printSummary = (lines: string[]): void => {
  process.stdout.write(`${lines.join("\n")}\n`)
}
