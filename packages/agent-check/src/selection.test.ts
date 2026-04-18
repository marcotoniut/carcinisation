import assert from "node:assert/strict"
import test from "node:test"
import { resolveRequestedFlags } from "./selection.js"

test("surface quick profile resolves to recommended flags", () => {
  const flags = resolveRequestedFlags([
    "--surface",
    "rust",
    "--profile",
    "quick",
  ])
  assert.deepEqual([...flags], ["--lint", "--fmt"])
})

test("surface full profile resolves to full surface flags", () => {
  const flags = resolveRequestedFlags([
    "--surface",
    "rust",
    "--profile",
    "full",
  ])
  assert.deepEqual([...flags], ["--lint", "--test", "--fmt"])
})

test("surface without profile resolves to all surface flags", () => {
  const flags = resolveRequestedFlags(["--surface", "rust"])
  assert.deepEqual(
    [...flags],
    ["--lint", "--test", "--fmt", "--clippy-pedantic"],
  )
})

test("multiple surfaces combine flags", () => {
  const flags = resolveRequestedFlags([
    "--surface",
    "rust",
    "--surface",
    "web",
    "--profile",
    "quick",
  ])
  assert.deepEqual([...flags], ["--lint", "--fmt", "--lint-biome"])
})

test("explicit flags and surface flags are merged", () => {
  const flags = resolveRequestedFlags([
    "--surface",
    "web",
    "--profile",
    "quick",
    "--test",
  ])
  assert.deepEqual([...flags], ["--test", "--lint-biome"])
})

test("profile without surface is rejected", () => {
  assert.throws(
    () => resolveRequestedFlags(["--profile", "quick"]),
    /requires at least one --surface/,
  )
})

test("control flags do not become requested checks", () => {
  const flags = resolveRequestedFlags([
    "--surface",
    "web",
    "--profile",
    "quick",
    "--json",
    "--fail-fast",
    "--instructionless",
    "--list",
  ])
  assert.deepEqual([...flags], ["--lint-biome"])
})
