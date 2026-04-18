import assert from "node:assert/strict"
import test from "node:test"
import { buildFocusLines } from "./utils.js"

test("summarises rust diagnostics with location and lint id", () => {
  const content = `
error: this function could have a \`#[must_use]\` attribute
  --> crates/asset_pipeline/src/analysis.rs:96:8
   |
   = help: for further information visit https://rust-lang.github.io/rust-clippy/rust-1.93.0/index.html#must_use_candidate
   = note: \`-D clippy::must-use-candidate\` implied by \`-D warnings\`
   = help: to override \`-D warnings\` add \`#[allow(clippy::must_use_candidate)]\`
`

  const focus = buildFocusLines({
    content,
    focusStyle: "rustc",
    matchers: [/error/i],
  })

  assert.deepEqual(focus, [
    "crates/asset_pipeline/src/analysis.rs:96:8 | error: this function could have a `#[must_use]` attribute | [clippy::must-use-candidate] | help: to override `-D warnings` add `#[allow(clippy::must_use_candidate)]`",
  ])
})

test("summarises cargo test failures with test name and location", () => {
  const content = `
---- builders::thumbnail::tests::fills_defaults stdout ----
thread 'builders::thumbnail::tests::fills_defaults' panicked at tools/editor/src/builders/thumbnail.rs:47:9:
assertion \`left == right\` failed
note: run with \`RUST_BACKTRACE=1\` environment variable to display a backtrace

failures:
    builders::thumbnail::tests::fills_defaults

test result: FAILED. 10 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
`

  const focus = buildFocusLines({
    content,
    focusStyle: "cargo-test",
    matchers: [/FAILED/i],
  })

  assert.deepEqual(focus, [
    "builders::thumbnail::tests::fills_defaults | tools/editor/src/builders/thumbnail.rs:47:9 | assertion `left == right` failed",
    "test result: FAILED. 10 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s",
  ])
})

test("summarises biome diagnostics without code frames", () => {
  const content = `
packages/agent-check/biome-fixture.js:1:7 lint/correctness/noUnusedVariables  FIXABLE  ━━━━━━━━━━━━━

  × This variable x is unused.

packages/agent-check/biome-fixture.js:2:1 parse ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  × Expected an expression, or an assignment but instead found the end of the file.

Found 4 errors.
`

  const focus = buildFocusLines({
    content,
    focusStyle: "biome",
    matchers: [/error/i],
  })

  assert.deepEqual(focus, [
    "packages/agent-check/biome-fixture.js:1:7 | lint/correctness/noUnusedVariables | This variable x is unused.",
    "packages/agent-check/biome-fixture.js:2:1 | parse | Expected an expression, or an assignment but instead found the end of the file.",
    "Found 4 errors.",
  ])
})

test("summarises biome format diagnostics without line numbers", () => {
  const content = `
packages/agent-check/src/utils.ts format ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  × Formatter would have printed the following content:

Found 1 error.
`

  const focus = buildFocusLines({
    content,
    focusStyle: "biome",
    matchers: [/error/i],
  })

  assert.deepEqual(focus, [
    "packages/agent-check/src/utils.ts | format | Formatter would have printed the following content:",
    "Found 1 error.",
  ])
})

test("prefers diff headers for rust-or-diff", () => {
  const content = `
cargo fmt --all -- --check
Diff in crates/carapace/src/sprite.rs:10:
@@ -1,3 +1,3 @@
`

  const focus = buildFocusLines({
    content,
    focusStyle: "rust-or-diff",
    matchers: [/Diff in/i],
  })

  assert.deepEqual(focus, ["Diff in crates/carapace/src/sprite.rs:10:"])
})
