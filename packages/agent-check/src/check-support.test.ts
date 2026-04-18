import assert from "node:assert/strict"
import test from "node:test"
import { classifyFailure } from "./check-support.js"

test("classifies missing commands as environment failures", () => {
  assert.equal(classifyFailure("sh: biome: command not found"), "environment")
})

test("classifies invalid cli usage as tool failures", () => {
  assert.equal(
    classifyFailure(
      "invalid profile 'fast' (expected one of: quick, full, advisory)",
    ),
    "tool",
  )
})

test("classifies lint/test output as code failures", () => {
  assert.equal(
    classifyFailure("src/foo.ts:1:1 | parse | Expected an expression."),
    "code",
  )
})

test("does not treat generic runtime messages as environment failures", () => {
  assert.equal(
    classifyFailure(
      "thread 'connect_test' panicked at tests/socket.rs:14:9: Connection refused",
    ),
    "code",
  )
  assert.equal(
    classifyFailure(
      "thread 'fs_test' panicked at tests/fs.rs:8:5: No such file or directory",
    ),
    "code",
  )
})
