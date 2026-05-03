//! CLI wrapper for the parser-based Aseprite piece-atlas exporter.
//!
//! This binary resolves command-line arguments and delegates all export logic
//! to `asset_pipeline::aseprite`.

use anyhow::{Context, Result, bail};
use asset_pipeline::aseprite::{
    ExportRequest, SimpleAtlasRequest, default_manifest_path, default_output_root,
    export_simple_atlas, export_simple_atlas_manifest, export_sprite,
};
use std::path::PathBuf;

fn main() -> Result<()> {
    // Spawn on a thread with a larger stack to accommodate the 16MB palette LUT
    // and deep aseprite-loader call chains.
    let child = std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(run)
        .expect("failed to spawn worker thread");
    child.join().unwrap()
}

fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(std::string::String::as_str) {
        Some("--simple-atlas") => {
            let request = parse_simple_atlas_args(&args[1..])?;
            export_simple_atlas(&request)
        }
        Some("--simple-atlases") => {
            let (manifest, assets_root) = parse_simple_atlases_args(&args[1..])?;
            export_simple_atlas_manifest(&manifest, &assets_root)
        }
        _ => {
            let request = parse_args(&args)?;
            export_sprite(&request)
        }
    }
}

fn parse_simple_atlas_args(args: &[String]) -> Result<SimpleAtlasRequest> {
    let mut aseprite_path = None;
    let mut output_dir = None;
    let mut pxi_asset_path = None;
    let mut layer_filter = None;
    let mut bounds_layer_filter = None;
    let mut tag_filter = None;

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--input" => {
                aseprite_path = Some(PathBuf::from(iter.next().context("Missing --input value")?));
            }
            "--output" => {
                output_dir = Some(PathBuf::from(
                    iter.next().context("Missing --output value")?,
                ));
            }
            "--pxi-asset-path" => {
                pxi_asset_path = Some(PathBuf::from(
                    iter.next().context("Missing --pxi-asset-path value")?,
                ));
            }
            "--layer" => {
                layer_filter = Some(iter.next().context("Missing --layer value")?.clone());
            }
            "--bounds-layer" => {
                bounds_layer_filter =
                    Some(iter.next().context("Missing --bounds-layer value")?.clone());
            }
            "--tag" => {
                tag_filter = Some(iter.next().context("Missing --tag value")?.clone());
            }
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            unknown => bail!("Unknown simple-atlas flag: {unknown}"),
        }
    }

    let output = output_dir.context("Missing required --output")?;
    let pxi_asset = pxi_asset_path.unwrap_or_else(|| output.join("atlas.pxi"));

    Ok(SimpleAtlasRequest {
        aseprite_path: aseprite_path.context("Missing required --input")?,
        output_dir: output,
        pxi_asset_path: pxi_asset,
        layer_filter,
        bounds_layer_filter,
        tag_filter,
    })
}

fn parse_simple_atlases_args(args: &[String]) -> Result<(PathBuf, PathBuf)> {
    let mut manifest = None;
    let mut assets_root = PathBuf::from("assets");

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--manifest" => {
                manifest = Some(PathBuf::from(
                    iter.next().context("Missing --manifest value")?,
                ));
            }
            "--assets-root" => {
                assets_root = PathBuf::from(iter.next().context("Missing --assets-root value")?);
            }
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            unknown => bail!("Unknown simple-atlases flag: {unknown}"),
        }
    }

    Ok((
        manifest.context("Missing required --manifest")?,
        assets_root,
    ))
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
    println!("  Composed enemy:");
    println!(
        "    cargo run -p process-aseprite -- --entity mosquiton --depth 3 [--manifest ...] [--output-root ...]"
    );
    println!("  Single simple atlas:");
    println!(
        "    cargo run -p process-aseprite -- --simple-atlas --input file.aseprite --output dir/"
    );
    println!("  Batch simple atlases from manifest:");
    println!(
        "    cargo run -p process-aseprite -- --simple-atlases --manifest resources/sprites/attacks/data.toml [--assets-root assets]"
    );
}
