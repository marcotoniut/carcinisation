import fs from "node:fs"
import path from "node:path"
import { fileURLToPath } from "node:url"
import type { ValidationSurface } from "./config.js"
import {
  DEFAULT_FLAGS,
  RULES,
  SURFACE_FLAGS,
  SURFACE_PROFILES,
  TASKS,
} from "./config.js"

type CompactPayload = {
  version: number
  defaultFlags: string[]
  flags: string[]
  focusFiles: Record<string, string>
  surfaceFlags: Record<ValidationSurface, string[]>
  surfaceProfiles: Record<
    ValidationSurface,
    {
      quick: string[]
      full: string[]
      advisory: string[]
    }
  >
  rules: string[]
}

type PrettyPayload = CompactPayload & {
  tasks: Array<{
    flag: string
    name: string
    command: string
    logFile: string
    focusFile: string
    phase: "parallel" | "post"
  }>
}

const scriptDir = path.dirname(fileURLToPath(import.meta.url))
const repoRoot = path.resolve(scriptDir, "../../..")

const DEFAULT_OUTPUT_PATH = path.resolve(
  repoRoot,
  ".direnv",
  "cache",
  "check-agent-capabilities.json",
)

const toPrettyPath = (jsonPath: string): string =>
  jsonPath.endsWith(".json")
    ? `${jsonPath.slice(0, -".json".length)}.pretty.json`
    : `${jsonPath}.pretty.json`

const parseArgs = (): { outPath: string } => {
  const args = process.argv.slice(2)
  const outIndex = args.indexOf("--out")
  const outArg = outIndex >= 0 ? args.at(outIndex + 1) : undefined
  const rawOut = outArg ?? DEFAULT_OUTPUT_PATH
  const outPath = path.isAbsolute(rawOut)
    ? rawOut
    : path.resolve(repoRoot, rawOut)
  return { outPath }
}

const buildCompactPayload = (): CompactPayload => ({
  version: 1,
  defaultFlags: [...DEFAULT_FLAGS],
  flags: TASKS.map((task) => task.flag),
  focusFiles: Object.fromEntries(
    TASKS.map((task) => [task.flag, task.focusFile]),
  ),
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
  ) as CompactPayload["surfaceProfiles"],
  rules: [...RULES],
})

const buildPrettyPayload = (): PrettyPayload => ({
  ...buildCompactPayload(),
  tasks: TASKS.map((task) => ({
    flag: task.flag,
    name: task.name,
    command: task.command,
    logFile: task.logFile,
    focusFile: task.focusFile,
    phase: task.phase,
  })),
})

const main = () => {
  const { outPath } = parseArgs()
  const prettyPath = toPrettyPath(outPath)
  const compactPayload = buildCompactPayload()
  const prettyPayload = buildPrettyPayload()

  fs.mkdirSync(path.dirname(outPath), { recursive: true })
  const outTmp = `${outPath}.tmp`
  const prettyTmp = `${prettyPath}.tmp`
  fs.writeFileSync(outTmp, `${JSON.stringify(compactPayload)}\n`, "utf8")
  fs.writeFileSync(
    prettyTmp,
    `${JSON.stringify(prettyPayload, null, 2)}\n`,
    "utf8",
  )
  fs.renameSync(outTmp, outPath)
  fs.renameSync(prettyTmp, prettyPath)

  console.log(
    `[check-agent-capabilities] wrote compact ${path.relative(repoRoot, outPath)} and full ${path.relative(repoRoot, prettyPath)}`,
  )
}

main()
