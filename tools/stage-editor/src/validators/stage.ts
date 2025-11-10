/**
 * Stage Data Validator
 *
 * Performs basic structural validation on stage data loaded from RON files.
 * The Rust RON deserializer handles full validation - this is just a safety check.
 */

import type { StageData } from "@/types/generated/StageData"

export interface ValidationSuccess<T> {
  success: true
  data: T
}

export interface ValidationFailure {
  success: false
  message: string
}

export type ValidationResult<T> = ValidationSuccess<T> | ValidationFailure

/**
 * Validate unknown data as StageData
 *
 * Performs basic structural checks. Full validation happens in Rust via RON deserialization.
 *
 * @param data - Unknown data (typically from JSON.parse or RON bridge)
 * @returns Validated StageData or validation error
 */
export function validateStageData(data: unknown): ValidationResult<StageData> {
  try {
    if (!data || typeof data !== "object") {
      throw new Error("StageData must be an object")
    }

    const obj = data as Record<string, unknown>

    // Check required fields exist
    if (typeof obj.name !== "string") {
      throw new Error("StageData.name must be a string")
    }
    if (typeof obj.background_path !== "string") {
      throw new Error("StageData.background_path must be a string")
    }
    if (typeof obj.music_path !== "string") {
      throw new Error("StageData.music_path must be a string")
    }
    if (!Array.isArray(obj.spawns)) {
      throw new Error("StageData.spawns must be an array")
    }
    if (!Array.isArray(obj.steps)) {
      throw new Error("StageData.steps must be an array")
    }

    return {
      success: true,
      data: data as StageData,
    }
  } catch (error) {
    return {
      success: false,
      message: error instanceof Error ? error.message : String(error),
    }
  }
}
