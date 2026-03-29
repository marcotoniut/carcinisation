//! CLI wrapper for the parser-based Aseprite piece-atlas exporter.
//!
//! This binary resolves command-line arguments and delegates all export logic
//! to `asset_pipeline::aseprite`.

use anyhow::{Context, Result, bail};
use asset_pipeline::aseprite::{
    ExportRequest, default_manifest_path, default_output_root, export_sprite,
};
use std::path::PathBuf;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let request = parse_args(&args)?;
    export_sprite(&request)
}

fn parse_args(args: &[String]) -> Result<ExportRequest> {
    let mut manifest_path = default_manifest_path();
    let mut output_root = default_output_root();
    let mut entity = None;
    let mut depth = None;

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--manifest" => {
                let value = iter.next().context("Missing value for --manifest")?;
                manifest_path = PathBuf::from(value);
            }
            "--entity" => {
                entity = Some(iter.next().context("Missing value for --entity")?.clone());
            }
            "--depth" => {
                let value = iter.next().context("Missing value for --depth")?;
                depth = Some(
                    value
                        .parse::<u8>()
                        .with_context(|| format!("Invalid depth '{value}'"))?,
                );
            }
            "--output-root" => {
                let value = iter.next().context("Missing value for --output-root")?;
                output_root = PathBuf::from(value);
            }
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            unknown => bail!("Unknown process-aseprite flag: {unknown}"),
        }
    }

    Ok(ExportRequest {
        manifest_path,
        entity: entity.context("Missing required --entity")?,
        depth: depth.context("Missing required --depth")?,
        output_root,
    })
}

fn print_usage() {
    println!("Usage:");
    println!(
        "  cargo run -p process-aseprite -- --entity mosquiton --depth 3 [--manifest resources/sprites/data.toml] [--output-root tmp/aseprite-export]"
    );
}
