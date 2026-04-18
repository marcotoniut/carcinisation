export type FailureType = "code" | "tool" | "environment"

const ENVIRONMENT_FAILURE_MATCHERS = [
  /Cannot connect to the Docker daemon/i,
  /Executable doesn't exist at .*ms-playwright/i,
  /is already used, make sure that nothing is running on the port/i,
  /^(?:sh|bash|zsh): .*command not found$/im,
  /^(?:sh|bash|zsh): .*permission denied$/im,
  /^(?:sh|bash|zsh): .*No such file or directory$/im,
  /\bspawn\s+.+\s+(?:EACCES|EPERM)\b/i,
  /\bspawn\s+.+\s+ENOENT\b/i,
  /\bgetaddrinfo\s+ENOTFOUND\b/i,
  /Temporary failure in name resolution/i,
  /\bconnect\s+ECONNREFUSED\b/i,
  /No space left on device/i,
] as const

const TOOL_FAILURE_MATCHERS = [
  /missing value for --surface/i,
  /missing value for --profile/i,
  /invalid surface /i,
  /invalid profile /i,
  /--profile requires at least one --surface/i,
  /check: FAIL \(runner error\)/i,
] as const

export const classifyFailure = (output: string): FailureType => {
  if (ENVIRONMENT_FAILURE_MATCHERS.some((matcher) => matcher.test(output))) {
    return "environment"
  }

  if (TOOL_FAILURE_MATCHERS.some((matcher) => matcher.test(output))) {
    return "tool"
  }

  return "code"
}
