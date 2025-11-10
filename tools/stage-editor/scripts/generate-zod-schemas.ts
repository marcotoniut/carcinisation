#!/usr/bin/env tsx

/**
 * Generate Zod schemas from TypeScript types using ts-to-zod.
 *
 * Input: src/types/generated/*.ts
 * Output: src/types/schemas/*.zod.ts + index.ts barrel file
 *
 * Note: ts-to-zod fails on complex discriminated unions.
 * Exits success if at least one schema generates successfully.
 */

import { execSync } from "node:child_process"
import {
  existsSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs"
import { basename, dirname, join, relative } from "node:path"
import { fileURLToPath } from "node:url"

const __filename = fileURLToPath(import.meta.url)
const __dirname = dirname(__filename)
const editorRoot = join(__dirname, "..")

const TYPES_DIR = join(editorRoot, "src/types/generated")
const SCHEMAS_DIR = join(editorRoot, "src/types/schemas")
const BANNER =
  "// ⚠️ Auto-generated. Do not edit. Source of truth: Rust types.\n\n"
const QUIET = process.env.QUIET === "1"

// Clean and create schemas directory
if (existsSync(SCHEMAS_DIR)) {
  rmSync(SCHEMAS_DIR, { recursive: true, force: true })
}
mkdirSync(SCHEMAS_DIR, { recursive: true })

if (!QUIET) {
  console.log("⚡ Generating Zod schemas from TypeScript types...")
}

// Get all TypeScript files
const typeFiles = readdirSync(TYPES_DIR)
  .filter((f) => f.endsWith(".ts"))
  .sort()

let generated = 0
let failed = 0

// Generate Zod schema for each type file
for (const file of typeFiles) {
  const baseName = basename(file, ".ts")
  const inputPath = join(TYPES_DIR, file)
  const outputPath = join(SCHEMAS_DIR, `${baseName}.zod.ts`)

  // Use relative paths for ts-to-zod
  const relativeInput = relative(editorRoot, inputPath)
  const relativeOutput = relative(editorRoot, outputPath)

  try {
    // Run ts-to-zod for this file with relative paths and cwd set to editorRoot
    execSync(
      `npx ts-to-zod "${relativeInput}" "${relativeOutput}" --keepComments`,
      {
        cwd: editorRoot,
        stdio: "pipe", // Suppress output
      },
    )

    // Read the generated file and add banner
    let content = readFileSync(outputPath, "utf-8")

    // Add banner if not present
    if (!content.startsWith("// ⚠️")) {
      content = BANNER + content
    }

    // Fix imports to use relative paths from schemas dir
    content = content
      .replace(/from ['"]\.\.\/types\//g, "from '../generated/types/")
      .replace(
        /from ['"]\.\.\/\.\.\/generated\/types\//g,
        "from '../generated/types/",
      )

    writeFileSync(outputPath, content)
    generated++
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error)
    console.error(`  ❌ Failed to generate ${baseName}: ${message}`)
    failed++
  }
}

// Generate barrel file (index.ts) - only export successfully generated schemas
const exports = typeFiles
  .filter((f) => {
    const baseName = basename(f, ".ts")
    const schemaPath = join(SCHEMAS_DIR, `${baseName}.zod.ts`)
    return existsSync(schemaPath)
  })
  .map((f) => {
    const baseName = basename(f, ".ts")
    return `export * from './${baseName}.zod'`
  })
  .join("\n")

const barrelContent = `${BANNER}${exports}\n`
writeFileSync(join(SCHEMAS_DIR, "index.ts"), barrelContent)

// Report results
if (generated === 0) {
  console.error(`❌ Failed to generate any Zod schemas (${failed} failures)`)
  process.exit(1)
} else if (failed > 0) {
  if (QUIET) {
    console.log(
      `⚠️  ${generated}/${typeFiles.length} Zod schemas (${failed} failed)`,
    )
  } else {
    console.log(
      `⚠️  Generated ${generated}/${typeFiles.length} Zod schemas (${failed} failures)`,
    )
    console.log(
      `  Generated barrel file: index.ts (${typeFiles.length} exports)`,
    )
    console.log(
      `  Note: Some types are too complex for ts-to-zod. Consider hand-writing schemas for failed types.`,
    )
  }
} else {
  if (QUIET) {
    console.log(`✅ ${generated} Zod schemas`)
  } else {
    console.log(`✅ Generated ${generated} Zod schemas`)
    console.log(
      `  Generated barrel file: index.ts (${typeFiles.length} exports)`,
    )
  }
}
