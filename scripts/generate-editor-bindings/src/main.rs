//! Generate TypeScript definitions from the runtime data model.
//! This binary drives ts-rs exports directly into the stage editor.

use anyhow::{Context, Result};
use std::{
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
    time::Instant,
};

#[cfg(feature = "derive-ts")]
use carcinisation::stage::data::StageData;
#[cfg(feature = "derive-ts")]
use ts_rs::TS;

fn main() {
    if let Err(error) = run() {
        eprintln!("❌ Error generating editor types: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let quiet = env::var("QUIET").is_ok();

    let ts_out = PathBuf::from(
        env::var("TS_OUT").unwrap_or_else(|_| "tools/stage-editor/src/types/generated".into()),
    );

    if ts_out.exists() {
        fs::remove_dir_all(&ts_out).context("failed to clear TS output directory")?;
    }
    fs::create_dir_all(&ts_out).context("failed to create TS output directory")?;

    let start = Instant::now();

    // Always print start message so grep can detect it
    println!("⚡ Generating TypeScript from Rust types (ts-rs)...");

    export_types(&ts_out)?;

    if !quiet {
        print!("  Fixing imports");
        io::stdout().flush().ok();
    }

    // Post-process to fix missing imports from inline type annotations
    fix_missing_imports(&ts_out, quiet)?;

    if !quiet {
        println!(" ✓");
    }

    // Always print success message so grep can detect it
    println!(
        "✅ TS generated to {} in {:.2}s",
        ts_out.display(),
        start.elapsed().as_secs_f32()
    );

    Ok(())
}

#[cfg(feature = "derive-ts")]
fn export_types(ts_out: &Path) -> Result<()> {
    StageData::export_all_to(ts_out).context("failed to export TypeScript bindings via ts-rs")
}

#[cfg(not(feature = "derive-ts"))]
fn export_types(_: &Path) -> Result<()> {
    anyhow::bail!("derive-ts feature required to export TypeScript types")
}

fn fix_missing_imports(ts_dir: &Path, quiet: bool) -> Result<()> {
    // Map of files that need import fixes: filename -> (type_ref, type_name)
    let fixes: &[(&str, &[(&str, &str)])] = &[
        ("EnemyDropSpawn.ts", &[("EnemyStep", "EnemyStep")]),
        (
            "EnemySpawn.ts",
            &[("EnemyStep", "EnemyStep"), ("Depth", "Depth")],
        ),
        ("TweenStageStep.ts", &[("Depth", "Depth")]),
        ("StopStageStep.ts", &[("Depth", "Depth")]),
    ];

    for (idx, (filename, imports)) in fixes.iter().enumerate() {
        let file_path = ts_dir.join(filename);
        if !file_path.exists() {
            continue;
        }

        let content = fs::read_to_string(&file_path)?;

        // Check which types are actually used but not imported
        let mut needed_imports = vec![];
        for (type_ref, type_name) in *imports {
            let import_statement = format!("from \"./{type_name}\"");
            if content.contains(type_ref) && !content.contains(&import_statement) {
                needed_imports.push(format!(
                    "import type {{ {type_name} }} from \"./{type_name}\";"
                ));
            }
        }

        if needed_imports.is_empty() {
            continue;
        }

        // Find where to insert imports (after the last existing import or after first line)
        let lines: Vec<String> = content.lines().map(String::from).collect();
        let import_end_idx = lines
            .iter()
            .position(|line| line.starts_with("import "))
            .map(|first_import_idx| {
                lines[first_import_idx..]
                    .iter()
                    .rposition(|line| line.starts_with("import "))
                    .map(|rel_idx| first_import_idx + rel_idx + 1)
                    .unwrap_or(first_import_idx + 1)
            })
            .unwrap_or(1);

        let mut new_lines: Vec<String> = lines[..import_end_idx].to_vec();
        new_lines.extend(needed_imports);
        new_lines.extend_from_slice(&lines[import_end_idx..]);

        let fixed_content = new_lines.join("\n") + "\n";
        fs::write(&file_path, fixed_content)?;

        if !quiet {
            let dots_count = (idx % 3) + 1;
            let dots = ".".repeat(dots_count);
            let spaces = " ".repeat(3 - dots_count);
            print!("\r  Fixing imports{}{}", dots, spaces);
            io::stdout().flush().ok();
        }
    }

    Ok(())
}
