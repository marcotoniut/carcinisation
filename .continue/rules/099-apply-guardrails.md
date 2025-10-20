# General Guardrails

- Target changes to the minimal set of files, but include new files when they serve the feature (call this out explicitly).
- Couple docs/tests with the code they describe; avoid doc-only diffs unless you are specifically polishing documentation.
- Preserve existing behavior unless a feature request requires otherwise; document intentional changes in comments or commit notes.
- Keep WASM nuance in mind: `bevy/dynamic_linking` stays native-only—do not remove it from desktop configs to appease wasm builds.
- When in doubt, surface a plan before large refactors and prefer iterative, testable steps.
