use std::fs::File;
use std::io::Write;

use bevy::tasks::AsyncComputeTaskPool;

use super::constants::RECENT_FILE_PATH;

pub fn write_recent_file_path_file(path: String) {
    let thread_pool = AsyncComputeTaskPool::get();
    thread_pool
        .spawn(async move {
            if let Ok(mut file) = File::create(RECENT_FILE_PATH) {
                if let Err(e) = writeln!(file, "{}", path) {
                    eprintln!("Failed to write to recent file path: {:?}", e);
                }
            } else {
                eprintln!("Failed to create recent file path");
            }
        })
        .detach();
}
