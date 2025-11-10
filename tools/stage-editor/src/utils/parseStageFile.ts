import type { StageData } from "@/types/generated/StageData"
import { validateStageData } from "../validators/stage"

export class ParseError extends Error {
  constructor(
    message: string,
    public context?: string,
  ) {
    super(message)
    this.name = "ParseError"
  }
}

export async function parseStageFile(ronText: string): Promise<StageData> {
  const response = await fetch("/api/ron-to-json", {
    method: "POST",
    headers: {
      "Content-Type": "text/plain",
    },
    body: ronText,
  })

  const responseText = await response.text()

  if (!response.ok) {
    const errorMessage = extractErrorMessage(responseText)
    throw new ParseError(errorMessage, "Failed to convert RON to JSON")
  }

  let parsedJson: unknown
  try {
    parsedJson = JSON.parse(responseText)
  } catch (error) {
    throw new ParseError(
      "Conversion service returned invalid JSON",
      error instanceof Error ? error.message : String(error),
    )
  }

  const validation = validateStageData(parsedJson)
  if (validation.success) {
    return validation.data
  }
  throw new ParseError("Stage data validation failed", validation.message)
}

function extractErrorMessage(errorText: string): string {
  if (!errorText) {
    return "ron bridge request failed"
  }

  try {
    const parsed = JSON.parse(errorText)
    if (parsed?.error) {
      return String(parsed.error)
    }
  } catch {
    // ignore
  }

  return errorText
}
