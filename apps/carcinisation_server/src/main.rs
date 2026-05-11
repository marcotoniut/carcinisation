use std::path::PathBuf;
use std::time::Duration;

use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use carcinisation_fps_core::map::Map;
use carcinisation_server::ServerPlugin;
use clap::Parser;

/// Default map — must match the `multiplayer_client` default.
const DEFAULT_MAP: &str = "assets/config/fp/test_room.fp_map.ron";

#[derive(Parser, Clone, Debug)]
struct Args {
    #[arg(long, default_value = "5000")]
    port: u16,
    #[arg(long, default_value = DEFAULT_MAP)]
    map: PathBuf,
}

fn main() {
    let args = Args::parse();

    let map_ron = std::fs::read_to_string(&args.map)
        .unwrap_or_else(|e| panic!("failed to read map {}: {e}", args.map.display()));
    let map_data = Map::load_data(&map_ron)
        .unwrap_or_else(|e| panic!("failed to parse map {}: {e}", args.map.display()));

    let mut app = App::new();

    // Explicit plugin list instead of MinimalPlugins — configurable tick loop.
    // 1ms poll interval avoids busy-spinning while keeping latency low.
    app.add_plugins((
        bevy::app::TaskPoolPlugin::default(),
        bevy::diagnostic::FrameCountPlugin,
        bevy::time::TimePlugin,
        ScheduleRunnerPlugin::run_loop(Duration::from_millis(1)),
        bevy::app::TerminalCtrlCHandlerPlugin,
    ));
    app.add_plugins(bevy::log::LogPlugin::default());
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.add_plugins(ServerPlugin {
        port: args.port,
        map: map_data.map,
        entities: map_data.entities,
        player_starts: map_data.player_starts,
    });

    app.run();
    // Reached after Ctrl+C / AppExit.
    eprintln!("Server stopped.");
}
