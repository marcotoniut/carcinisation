//! Generate TypeScript type definitions and Zod schemas from Rust types.
//!
//! Pipeline: Rust → TypeScript (ts-rs) → Zod schemas (ts-to-zod)
//!
//! Output:
//!   - TypeScript types: tools/editor-v2/generated/types/*.ts
//!   - Zod schemas: tools/editor-v2/generated/schemas/*.zod.ts
//!
//! Usage:
//!   make dev-editor-v2    - Vite + cargo watch in parallel (recommended)
//!   make gen-types        - Generate once
//!   make watch-types      - Auto-regenerate on Rust changes
//!
//! Environment variables:
//!   QUIET=1      - Concise output (for watch mode)
//!   SKIP_ZOD=1   - Skip Zod generation
//!   TS_OUT=path  - TypeScript output directory
//!   ZOD_OUT=path - Zod output directory

#[cfg(not(feature = "derive-ts"))]
fn main() {
    eprintln!("Error: This binary requires the 'derive-ts' feature flag.");
    eprintln!("Run with: cargo run --bin gen_types --features derive-ts");
    std::process::exit(1);
}

#[cfg(feature = "derive-ts")]
fn main() {
    if let Err(error) = run() {
        eprintln!("❌ Error generating editor types: {error}");
        std::process::exit(1);
    }
}

#[cfg(feature = "derive-ts")]
fn run() -> anyhow::Result<()> {
    use anyhow::{anyhow, Context};
    use carcinisation::{
        cutscene::data::{CutsceneAnimationSpawn, CutsceneAnimationsSpawn, TargetMovement},
        stage::{
            components::{placement::Depth, CinematicStageStep, MovementStageStep, StopStageStep},
            data::{
                ContainerSpawn, EnemyDropSpawn, EnemySpawn, ObjectSpawn, ObjectType,
                PickupDropSpawn, PickupSpawn, PickupType, SkyboxData, StageData, StageSpawn,
                StageStep,
            },
            destructible::{components::DestructibleType, data::DestructibleSpawn},
            enemy::{
                data::steps::{
                    AttackEnemyStep, CircleAroundEnemyStep, EnemyStep, IdleEnemyStep,
                    JumpEnemyStep, LinearMovementEnemyStep,
                },
                entity::EnemyType,
            },
        },
    };
    use std::{
        collections::HashSet,
        env, fs,
        path::{Path, PathBuf},
        process::Command,
        time::Instant,
    };
    use ts_rs::TS;

    const BANNER: &str = "// ⚠️ Auto-generated. Do not edit. Source of truth: Rust types.\n\n";

    #[derive(Clone, Copy)]
    enum ExportStatus {
        Created,
        Updated,
        Unchanged,
    }

    fn export_type<T: TS + ?Sized + 'static>(
        type_name: &str,
        output_dir: &Path,
    ) -> anyhow::Result<(ExportStatus, PathBuf)> {
        let relative_path = T::output_path()
            .ok_or_else(|| anyhow!("{} cannot be exported", std::any::type_name::<T>()))?;
        let target_path = output_dir.join(relative_path);

        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to prepare {}", parent.display()))?;
        }

        let mut new_contents = BANNER.to_string();
        new_contents.push_str(
            &T::export_to_string()
                .map_err(|error| anyhow!(error))
                .with_context(|| format!("failed to render {type_name}"))?,
        );

        let status = if target_path.exists() {
            let existing = fs::read_to_string(&target_path)
                .with_context(|| format!("failed to read {}", target_path.display()))?;
            if existing == new_contents {
                ExportStatus::Unchanged
            } else {
                fs::write(&target_path, new_contents)
                    .with_context(|| format!("failed to update {}", target_path.display()))?;
                ExportStatus::Updated
            }
        } else {
            fs::write(&target_path, new_contents)
                .with_context(|| format!("failed to write {}", target_path.display()))?;
            ExportStatus::Created
        };

        Ok((status, target_path))
    }

    fn remove_stale_files(root: &Path, expected: &HashSet<PathBuf>) -> anyhow::Result<usize> {
        let mut removed = 0usize;
        if !root.exists() {
            return Ok(removed);
        }

        let mut stack = vec![root.to_path_buf()];
        while let Some(dir) = stack.pop() {
            for entry in
                fs::read_dir(&dir).with_context(|| format!("failed to read {}", dir.display()))?
            {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    stack.push(path);
                    continue;
                }

                if path.extension().map_or(true, |ext| ext != "ts") {
                    continue;
                }

                if !expected.contains(&path) {
                    fs::remove_file(&path)
                        .with_context(|| format!("failed to remove {}", path.display()))?;
                    removed += 1;
                }
            }
        }

        Ok(removed)
    }

    /// Generate Zod schemas from TypeScript types using ts-to-zod.
    ///
    /// Runs Node.js script that:
    /// - Iterates generated/types/*.ts files
    /// - Generates corresponding *.zod.ts files via ts-to-zod
    /// - Adds banner comments and creates barrel file
    ///
    /// Note: ts-to-zod has limitations with complex discriminated unions.
    /// Some types may fail - hand-write schemas in src/schemas/manual/ as needed.
    fn generate_zod_schemas(editor_dir: &Path, quiet: bool) -> anyhow::Result<()> {
        println!("⚡ Generating zod schema from types...");

        // Script path relative to editor directory
        let script_path = editor_dir.join("scripts/generate-zod-schemas.ts");

        if !script_path.exists() {
            return Err(anyhow!("Script not found: {}", script_path.display()));
        }

        // Execute TypeScript script via tsx (using pnpm to ensure tsx is available)
        // Pass relative path since we set current_dir to editor_dir
        let output = Command::new("pnpm")
            .arg("exec")
            .arg("tsx")
            .arg("scripts/generate-zod-schemas.ts")
            .current_dir(editor_dir)
            .output()
            .context("failed to run generate-zod-schemas.ts")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Err(anyhow!("Zod schema generation failed:\n{stderr}\n{stdout}"));
        }

        if !quiet {
            let stdout = String::from_utf8_lossy(&output.stdout);
            print!("{}", stdout);
        }

        Ok(())
    }

    let quiet = env::var("QUIET").is_ok();
    let skip_zod = env::var("SKIP_ZOD").is_ok();
    let start_time = Instant::now();

    // Get output directories from env or use defaults
    let ts_out_str =
        env::var("TS_OUT").unwrap_or_else(|_| "tools/editor-v2/generated/types".to_string());
    let zod_out_str =
        env::var("ZOD_OUT").unwrap_or_else(|_| "tools/editor-v2/generated/schemas".to_string());

    let output_dir = PathBuf::from(&ts_out_str);
    let zod_out = PathBuf::from(&zod_out_str);
    let editor_dir = PathBuf::from("tools/editor-v2");

    fs::create_dir_all(&output_dir)
        .context("failed to create output directory for generated types")?;

    println!("⚡ Generating editor types from Rust...");

    let mut created = 0usize;
    let mut updated = 0usize;
    let mut unchanged = 0usize;
    let mut failed: Vec<String> = Vec::new();
    let mut expected_paths: HashSet<PathBuf> = HashSet::new();

    macro_rules! export_type_entry {
        ($label:literal, $ty:ty) => {
            match export_type::<$ty>($label, &output_dir) {
                Ok((status, path)) => {
                    match status {
                        ExportStatus::Created => created += 1,
                        ExportStatus::Updated => updated += 1,
                        ExportStatus::Unchanged => unchanged += 1,
                    }
                    expected_paths.insert(path);
                }
                Err(error) => {
                    failed.push(format!("{}: {error}", $label));
                }
            }
        };
    }

    // Placement
    export_type_entry!("Depth", Depth);
    // Stage components
    export_type_entry!("CinematicStageStep", CinematicStageStep);
    export_type_entry!("MovementStageStep", MovementStageStep);
    export_type_entry!("StopStageStep", StopStageStep);
    // Stage data
    export_type_entry!("SkyboxData", SkyboxData);
    export_type_entry!("ObjectType", ObjectType);
    export_type_entry!("PickupType", PickupType);
    export_type_entry!("ContainerSpawn", ContainerSpawn);
    export_type_entry!("PickupSpawn", PickupSpawn);
    export_type_entry!("PickupDropSpawn", PickupDropSpawn);
    export_type_entry!("ObjectSpawn", ObjectSpawn);
    export_type_entry!("EnemySpawn", EnemySpawn);
    export_type_entry!("EnemyDropSpawn", EnemyDropSpawn);
    export_type_entry!("StageSpawn", StageSpawn);
    export_type_entry!("StageStep", StageStep);
    export_type_entry!("StageData", StageData);
    // Enemy
    export_type_entry!("EnemyType", EnemyType);
    export_type_entry!("AttackEnemyStep", AttackEnemyStep);
    export_type_entry!("CircleAroundEnemyStep", CircleAroundEnemyStep);
    export_type_entry!("IdleEnemyStep", IdleEnemyStep);
    export_type_entry!("JumpEnemyStep", JumpEnemyStep);
    export_type_entry!("LinearMovementEnemyStep", LinearMovementEnemyStep);
    export_type_entry!("EnemyStep", EnemyStep);
    // Destructible
    export_type_entry!("DestructibleType", DestructibleType);
    export_type_entry!("DestructibleSpawn", DestructibleSpawn);
    // Cutscene
    export_type_entry!("TargetMovement", TargetMovement);
    export_type_entry!("CutsceneAnimationSpawn", CutsceneAnimationSpawn);
    export_type_entry!("CutsceneAnimationsSpawn", CutsceneAnimationsSpawn);

    let removed = remove_stale_files(&output_dir, &expected_paths)?;

    if !failed.is_empty() {
        for failure in &failed {
            eprintln!("❌ Failed to export {failure}");
        }
        return Err(anyhow!("failed to export {} type(s)", failed.len()));
    }

    let elapsed = start_time.elapsed().as_secs_f32();
    let total = created + updated + unchanged;

    let summary = format!(
        "✅ Successfully generated {total} editor TypeScript definitions \
         (created {created}, updated {updated}, unchanged {unchanged}, removed {removed}) \
         in {elapsed:.2}s"
    );

    // Generate Zod schemas if not skipped
    if !skip_zod {
        match generate_zod_schemas(&editor_dir, quiet) {
            Ok(()) => {
                if !quiet {
                    println!(
                        "✅ Successfully generated Zod schemas in {}",
                        zod_out.display()
                    );
                }
            }
            Err(error) => {
                if quiet {
                    eprintln!("⚠️ Zod schema generation failed: {error}");
                } else {
                    eprintln!("❌ Failed to generate Zod schemas: {error}");
                    return Err(error);
                }
            }
        }
    }

    println!("{summary}");
    if !quiet {
        println!(
            "Generated files available in {} ({} total)",
            output_dir.display(),
            total
        );
    }

    Ok(())
}
