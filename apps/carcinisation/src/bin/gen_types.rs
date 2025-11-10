//! Generate TypeScript definitions from the runtime data model.
//! This binary copies ts-rs generated types to the stage editor.

use anyhow::{Context, Result};
use std::{
    env, fs,
    path::{Path, PathBuf},
    time::Instant,
};

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

    fs::create_dir_all(&ts_out).context("failed to create TS output directory")?;

    let start = Instant::now();
    println!("⚡ Generating TypeScript from Rust types (ts-rs)…");

    // ts-rs exports are generated at compile time via the derive-ts feature
    // Types are exported to the crate-level bindings directory
    let bindings_dir = PathBuf::from("apps/carcinisation/bindings");
    if !bindings_dir.exists() {
        anyhow::bail!(
            "ts-rs bindings directory not found. Make sure types are built with --features derive-ts"
        );
    }

    // Copy generated types to the target directory
    copy_dir_recursive(&bindings_dir, &ts_out).context("failed to copy TypeScript bindings")?;

    // Post-process to fix missing imports from inline type annotations
    fix_missing_imports(&ts_out)?;

    if !quiet {
        println!(
            "✅ TS generated to {} in {:.2}s",
            ts_out.display(),
            start.elapsed().as_secs_f32()
        );
    }

    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if file_type.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

fn fix_missing_imports(ts_dir: &Path) -> Result<()> {
    // Map of files that need import fixes: filename -> (type_ref, type_name)
    let fixes: &[(&str, &[(&str, &str)])] = &[
        ("EnemyDropSpawn.ts", &[("EnemyStep", "EnemyStep")]),
        (
            "EnemySpawn.ts",
            &[("EnemyStep", "EnemyStep"), ("Depth", "Depth")],
        ),
        ("MovementStageStep.ts", &[("Depth", "Depth")]),
        ("StopStageStep.ts", &[("Depth", "Depth")]),
    ];

    for (filename, imports) in fixes {
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
    }

    Ok(())
}
