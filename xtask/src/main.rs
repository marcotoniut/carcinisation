//! A cargo-xtask tool for build-time tasks.
use anyhow::Result;
use attack_data::{
    config::AttackConfig,
    packed::{PackedAttackData, PackedHeader},
};
use clap::{Parser, Subcommand};
use glob::glob;
use log::{error, info, LevelFilter};
use std::{
    collections::HashMap,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Packs asset files into a binary format for release builds.
    PackAssets,
}

fn main() -> Result<()> {
    env_logger::Builder::new()
        .filter_level(LevelFilter::Info)
        .init();
    let cli = Cli::parse();

    match &cli.command {
        Commands::PackAssets => {
            pack_assets()?;
        }
    }

    Ok(())
}

fn pack_assets() -> Result<()> {
    info!("Starting asset packing process...");

    // We need to find the workspace root to correctly locate the assets folder.
    let workspace_root = get_workspace_root()?;
    let assets_dir = workspace_root.join("assets");
    let ron_glob_path = assets_dir.join("attacks/*.ron");
    let output_path = assets_dir.join("attacks/attacks.bin");

    info!("Searching for attack configs in: {:?}", ron_glob_path);

    let mut attacks = HashMap::new();
    let mut loaded_count = 0;

    for entry in glob(ron_glob_path.to_str().unwrap())? {
        let path = entry?;
        let file_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        info!("Processing: {}", path.display());
        let ron_string = fs::read_to_string(&path)?;
        let config: AttackConfig = match ron::from_str(&ron_string) {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to parse {}: {}", path.display(), e);
                // Fail the build if any config is invalid.
                return Err(e.into());
            }
        };

        // Validate that the file name matches the ID in the config.
        if config.attack_id != file_name {
            let err_msg = format!(
                "Validation failed for {}: attack_id '{}' does not match file name '{}'.",
                path.display(),
                config.attack_id,
                file_name
            );
            error!("{}", err_msg);
            return Err(anyhow::anyhow!(err_msg));
        }

        attacks.insert(config.attack_id.clone(), config);
        loaded_count += 1;
    }

    if loaded_count > 0 {
        let packed_data = PackedAttackData {
            header: PackedHeader::new(),
            attacks,
        };

        let encoded: Vec<u8> = bincode::serialize(&packed_data)?;

        let mut file = File::create(&output_path)?;
        file.write_all(&encoded)?;
        info!(
            "Successfully packed {} attack configs into {}",
            loaded_count,
            output_path.display()
        );
    } else {
        info!("No attack configs found to pack.");
    }

    Ok(())
}

/// Finds the workspace root by looking for the root Cargo.toml.
fn get_workspace_root() -> Result<PathBuf> {
    let mut current_dir = std::env::current_dir()?;
    loop {
        if current_dir.join("Cargo.toml").exists() {
            let toml_content = fs::read_to_string(current_dir.join("Cargo.toml"))?;
            if toml_content.contains("[workspace]") {
                return Ok(current_dir);
            }
        }
        if !current_dir.pop() {
            return Err(anyhow::anyhow!(
                "Could not find workspace root (Cargo.toml with [workspace])."
            ));
        }
    }
}
