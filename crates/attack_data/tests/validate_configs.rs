//! Integration test that validates all attack configuration files.

use attack_data::config::AttackConfig;
use glob::glob;
use std::{fs, path::PathBuf};

/// Finds the workspace root by looking for the root Cargo.toml.
fn get_workspace_root() -> PathBuf {
    let mut current_dir = std::env::current_dir().unwrap();
    loop {
        if current_dir.join("Cargo.toml").exists() {
            let toml_content = fs::read_to_string(current_dir.join("Cargo.toml")).unwrap();
            if toml_content.contains("[workspace]") {
                return current_dir;
            }
        }
        if !current_dir.pop() {
            panic!("Could not find workspace root (Cargo.toml with [workspace]).");
        }
    }
}

#[test]
fn validate_all_attack_configs() {
    let root = get_workspace_root();
    let glob_path = root.join("assets/attacks/*.ron");
    let mut found_files = 0;

    for entry in glob(glob_path.to_str().unwrap()).expect("Failed to read glob pattern") {
        let path = entry.unwrap();
        found_files += 1;

        let ron_string = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", path.display(), e));

        let config: Result<AttackConfig, _> = ron::from_str(&ron_string);

        match config {
            Ok(cfg) => {
                let file_name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default();
                assert_eq!(
                    cfg.attack_id,
                    file_name,
                    "File name '{}' must match attack_id '{}' in {}",
                    file_name,
                    cfg.attack_id,
                    path.display()
                );
            }
            Err(e) => {
                panic!("Failed to parse {}: {}", path.display(), e);
            }
        }
    }

    assert!(
        found_files > 0,
        "No RON attack config files found in assets/attacks/"
    );
}
