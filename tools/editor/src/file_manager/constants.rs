use std::path::PathBuf;

pub const RECENT_FILE_NAME: &str = "recent_file_path.txt";

/// Returns the path to the recent file tracker under tools/editor.
pub fn recent_file_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(RECENT_FILE_NAME)
}
