import type { ValidationProfile, ValidationSurface } from "./config.js"
import {
  SURFACE_FLAGS,
  SURFACE_PROFILES,
  VALIDATION_PROFILES,
  VALIDATION_SURFACES,
} from "./config.js"

type ResolveRequestedFlagsOptions = {
  aliases?: Map<string, string>
}

const isValidationSurface = (value: string): value is ValidationSurface =>
  (VALIDATION_SURFACES as readonly string[]).includes(value)

const isValidationProfile = (value: string): value is ValidationProfile =>
  (VALIDATION_PROFILES as readonly string[]).includes(value)

export const resolveRequestedFlags = (
  args: string[],
  options: ResolveRequestedFlagsOptions = {},
): Set<string> => {
  const aliases = options.aliases ?? new Map<string, string>()
  const explicitFlags = new Set<string>()
  const surfaces: ValidationSurface[] = []
  let profile: ValidationProfile | undefined

  for (let i = 0; i < args.length; i += 1) {
    const arg = args[i]
    if (!arg.startsWith("--") || arg === "--") continue

    if (arg === "--instructionless") {
      continue
    }

    if (arg === "--json" || arg === "--fail-fast" || arg === "--list") {
      continue
    }

    if (arg === "--surface") {
      const value = args[i + 1]
      if (value === undefined) {
        throw new Error("missing value for --surface")
      }
      if (!isValidationSurface(value)) {
        throw new Error(
          `invalid surface '${value}' (expected one of: ${VALIDATION_SURFACES.join(", ")})`,
        )
      }
      surfaces.push(value)
      i += 1
      continue
    }

    if (arg === "--profile") {
      const value = args[i + 1]
      if (value === undefined) {
        throw new Error("missing value for --profile")
      }
      if (!isValidationProfile(value)) {
        throw new Error(
          `invalid profile '${value}' (expected one of: ${VALIDATION_PROFILES.join(", ")})`,
        )
      }
      profile = value
      i += 1
      continue
    }

    explicitFlags.add(aliases.get(arg) ?? arg)
  }

  if (profile !== undefined && surfaces.length === 0) {
    throw new Error("--profile requires at least one --surface")
  }

  if (surfaces.length > 0) {
    for (const surface of surfaces) {
      const impliedFlags =
        profile === undefined
          ? SURFACE_FLAGS[surface]
          : SURFACE_PROFILES[surface][profile]

      for (const flag of impliedFlags) {
        explicitFlags.add(flag)
      }
    }
  }

  return explicitFlags
}
