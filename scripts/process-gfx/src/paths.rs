use std::path::{Path, PathBuf};

pub const BASE_PALETTE_SUBPATH: &str = "palette/base.png";

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("scripts/process-gfx must live under the workspace root")
}

pub fn assets_path() -> PathBuf {
    workspace_root().join("assets")
}

pub fn resources_gfx_path() -> PathBuf {
    workspace_root().join("resources/gfx")
}
