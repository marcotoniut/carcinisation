# Rust Dependencies

- Always inspect `Cargo.toml` (workspace and crate-level) before suggesting dependency changes.
- Use Cargo tooling (`cargo add`, `cargo update`, `cargo tree`, etc.)—never `pip` or other language managers for Rust crates.
- Reference exact crate names (`bevy_framepace`, `seldom_pixel`, etc.) from the manifest to avoid typos or phantom packages.
- Keep the workspace (`Cargo.lock`) consistent across the game and tooling crates—update all impacted members.
- If unsure about a crate’s source, search the repo (`rg`, MCP tools) or check crates.io before recommending actions.
