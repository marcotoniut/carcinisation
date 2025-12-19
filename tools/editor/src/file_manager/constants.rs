use std::path::PathBuf;

pub const RECENT_FILE_NAME: &str = "recent_file_path.txt";

pub fn recent_file_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(RECENT_FILE_NAME)
}
