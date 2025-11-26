use std::{fs, path::PathBuf, process::ExitCode, sync::Arc};

use anyhow::{Context, Result};
use bevy::prelude::*;
use carcinisation::{
    app::{build_app, AppLaunchOptions, StartFlow},
    game::{
        events::GameStartupTrigger,
        resources::{GameData, GameProgress},
    },
    stage::{data::StageData, events::StageStartupTrigger},
};
use clap::Parser;
use serde::Deserialize;

const DEFAULT_CONFIG: &str = "apps/carcinisation/single-stage.config.ron";

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Launch a single stage directly for debugging."
)]
struct SingleStageArgs {
    /// Path to the single stage config (.ron) that points at the desired stage asset.
    #[arg(long = "stage-config", value_name = "PATH", default_value = DEFAULT_CONFIG)]
    stage_config: PathBuf,
}

#[derive(Debug, Deserialize)]
struct SingleStageConfig {
    stage_path: PathBuf,
}

struct SingleStageBootstrapPlugin {
    stage_data: Arc<StageData>,
    stage_path: PathBuf,
}

#[derive(Resource, Clone)]
struct LoadedSingleStage {
    stage_data: Arc<StageData>,
    stage_path: PathBuf,
}

impl Plugin for SingleStageBootstrapPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(LoadedSingleStage {
            stage_data: self.stage_data.clone(),
            stage_path: self.stage_path.clone(),
        })
        .add_systems(
            Startup,
            (
                log_stage_choice,
                install_single_stage_data,
                trigger_game_startup,
                trigger_stage_startup,
            )
                .chain(),
        );
    }
}

fn main() -> ExitCode {
    if let Err(err) = run() {
        eprintln!("single_stage failed: {err:?}");
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

fn run() -> Result<()> {
    let args = SingleStageArgs::parse();
    let single_stage_config = load_config(&args.stage_config)?;
    let stage_data = load_stage(&single_stage_config.stage_path)?;

    let mut app = build_app(AppLaunchOptions {
        start_flow: StartFlow::StageOnly,
    });
    app.add_plugins(SingleStageBootstrapPlugin {
        stage_data,
        stage_path: single_stage_config.stage_path,
    });
    app.run();
    Ok(())
}

fn load_config(path: &PathBuf) -> Result<SingleStageConfig> {
    let body = fs::read_to_string(path)
        .with_context(|| format!("failed to read single-stage config {}", path.display()))?;
    let config: SingleStageConfig = ron::from_str(&body)
        .with_context(|| format!("invalid single-stage config {}", path.display()))?;
    Ok(config)
}

fn load_stage(path: &PathBuf) -> Result<Arc<StageData>> {
    let body = fs::read_to_string(path)
        .with_context(|| format!("failed to read stage file {}", path.display()))?;
    let parsed: StageData =
        ron::from_str(&body).with_context(|| format!("invalid stage data {}", path.display()))?;
    Ok(Arc::new(parsed))
}

fn log_stage_choice(loaded: Res<LoadedSingleStage>) {
    info!(
        "Launching single-stage run with {}",
        loaded.stage_path.display()
    );
}

fn install_single_stage_data(mut commands: Commands, loaded: Res<LoadedSingleStage>) {
    commands.insert_resource(GameData {
        name: format!(
            "Single Stage ({})",
            loaded.stage_path.file_name().map_or_else(
                || loaded.stage_path.display().to_string(),
                |name| name.to_string_lossy().to_string()
            )
        ),
        steps: Vec::new(),
    });
    commands.insert_resource(GameProgress { index: 0 });
}

fn trigger_game_startup(mut commands: Commands) {
    commands.trigger(GameStartupTrigger);
}

fn trigger_stage_startup(loaded: Res<LoadedSingleStage>, mut commands: Commands) {
    commands.trigger(StageStartupTrigger {
        data: loaded.stage_data.clone(),
    });
}
