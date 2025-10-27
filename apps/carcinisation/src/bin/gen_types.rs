//! Generate TypeScript type definitions using ts-rs
//! Output: tools/editor-v2/src/types/generated/
//!
//! Run with: cargo run --bin gen_types --features derive-ts

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
        time::Instant,
    };
    use ts_rs::TS;

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

        let new_contents = T::export_to_string()
            .map_err(|error| anyhow!(error))
            .with_context(|| format!("failed to render {type_name}"))?;

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

    let quiet = env::var("QUIET").is_ok();
    let start_time = Instant::now();
    let output_dir = PathBuf::from("tools/editor-v2/generated/types");
    fs::create_dir_all(&output_dir)
        .context("failed to create output directory for generated types")?;

    if !quiet {
        println!("⚡ Generating editor types from Rust...");
    }

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

    if quiet {
        // Concise output for watch mode
        if created + updated + removed > 0 {
            println!(
                "⚡ Generated {total} types (+{created} ~{updated} -{removed}) in {elapsed:.2}s"
            );
        } else {
            println!("✅ {total} types unchanged ({elapsed:.2}s)");
        }
    } else {
        // Verbose output for manual runs
        println!(
            "✅ Successfully generated {total} editor TypeScript definitions \
             (created {created}, updated {updated}, unchanged {unchanged}, removed {removed}) \
             in {elapsed:.2}s"
        );
        println!(
            "Generated files available in tools/editor-v2/generated/types ({} total)",
            total
        );
    }

    Ok(())
}
