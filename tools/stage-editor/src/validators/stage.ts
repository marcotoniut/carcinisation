/**
 * Stage Data Validator
 *
 * Validates unknown data against Zod schemas
 * Returns typed StageData or validation errors
 */

import { ZodError, ZodIssue } from 'zod'
import { StageDataSchema, type StageData } from '../../generated/schemas/stage'

export interface ValidationSuccess<T> {
  success: true
  data: T
}

export interface ValidationFailure {
  success: false
  errors: ZodIssue[]
  message: string
}

export type ValidationResult<T> = ValidationSuccess<T> | ValidationFailure

/**
 * Validate unknown data as StageData
 *
 * @param data - Unknown data (typically from JSON.parse or RON bridge)
 * @returns Validated StageData or validation errors
 */
export function validateStageData(data: unknown): ValidationResult<StageData> {
  try {
    const validated = StageDataSchema.parse(data)
    return {
      success: true,
      data: validated,
    }
  } catch (error) {
    if (error instanceof ZodError) {
      return {
        success: false,
        errors: error.issues,
        message: formatZodErrors(error.issues),
      }
    }

    return {
      success: false,
      errors: [],
      message: error instanceof Error ? error.message : String(error),
    }
  }
}

/**
 * Format Zod errors into a human-readable message
 */
function formatZodErrors(issues: ZodIssue[]): string {
  return issues
    .map((issue) => {
      const path = issue.path.join('.')
      return `${path}: ${issue.message}`
    })
    .join('; ')
}
