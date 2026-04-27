//! Loops a cutscene for tuning/preview.
//!
//! Usage:
//!   cargo run --bin cutscene_loop             # loops the splash
//!   cargo run --bin cutscene_loop -- splash   # same

use bevy::prelude::*;
use carcinisation::{
    app::{AppLaunchOptions, StartFlow, build_app},
    cutscene::{
        data::CutsceneData,
        messages::{CutsceneShutdownEvent, CutsceneStartupEvent},
    },
    splash::systems::build_splash_cutscene_data,
};
use std::sync::Arc;

#[derive(Resource, Clone)]
struct LoopCutsceneData(Arc<CutsceneData>);

struct CutsceneLoopPlugin;

impl Plugin for CutsceneLoopPlugin {
    fn build(&self, app: &mut App) {
        // Prevent normal boot flow.
        app.insert_resource(carcinisation::resources::DevFlags {
            skip_splash: true,
            skip_menu: true,
            skip_cutscenes: false,
        });

        let (config, data) = build_splash_cutscene_data();

        #[cfg(debug_assertions)]
        if config.slowdown > 1 {
            app.insert_resource(carcinisation_core::core::time::TimeMultiplier::<
                carcinisation::cutscene::resources::CutsceneTimeDomain,
            >::new(1.0 / config.slowdown as f32));
        }
        let _ = config;

        app.insert_resource(LoopCutsceneData(Arc::new(data)));
        app.add_systems(PostStartup, fire_cutscene);
        app.add_observer(on_shutdown_restart);
    }
}

fn fire_cutscene(mut commands: Commands, data: Res<LoopCutsceneData>) {
    info!("Cutscene loop: starting");
    commands.trigger(CutsceneStartupEvent {
        data: data.0.clone(),
    });
}

fn on_shutdown_restart(
    _trigger: On<CutsceneShutdownEvent>,
    mut commands: Commands,
    data: Res<LoopCutsceneData>,
) {
    info!("Cutscene loop: restarting");
    commands.trigger(CutsceneStartupEvent {
        data: data.0.clone(),
    });
}

fn main() {
    // Uses StartFlow::Full to get CutscenePlugin registered. The DevFlags
    // override (skip_splash + skip_menu) causes on_post_startup to fire
    // GameStartupEvent, which activates GamePlugin. This is harmless —
    // the game progression system runs but has no stage data to load.
    // A dedicated StartFlow::CutsceneLoop would be cleaner but overkill
    // for a dev tool.
    let mut app = build_app(AppLaunchOptions {
        start_flow: StartFlow::Full,
        ..Default::default()
    });
    app.add_plugins(CutsceneLoopPlugin);
    app.run();
}
