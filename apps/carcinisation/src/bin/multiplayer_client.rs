use std::{fs, net::SocketAddr, path::PathBuf, process::ExitCode};

use bevy::prelude::*;
use carapace::prelude::*;
use carcinisation::first_person::FpsClientPlugin;
use carcinisation_fps::plugin::{Config, FpsAuthorityMode, FpsPlugin, PlayerDead, PlayerHealth};
use clap::Parser;
use serde::{Deserialize, Serialize};

const MAP_PATH: &str = "assets/config/fp/test_room.fp_map.ron";
const SKY_PATH: &str = "assets/config/sky/park.sky.ron";
const SCREEN_W: u32 = 160;
const SCREEN_H: u32 = 144;

#[derive(Parser, Debug)]
#[command(about = "Networked multiplayer FPS client - connects to server.")]
struct MpClientArgs {
    #[arg(long = "connect")]
    connect: Option<String>,
    #[arg(long = "map", default_value = MAP_PATH)]
    map_path: PathBuf,
    #[arg(long = "sky", default_value = SKY_PATH)]
    sky_path: PathBuf,
}

#[derive(Deserialize, Reflect, Serialize)]
#[px_layer]
enum Layer {
    Background,
    #[default]
    Main,
}

#[allow(dead_code, clippy::needless_pass_by_value)]
fn god_mode(config: Res<Config>, mut health: ResMut<PlayerHealth>, mut dead: ResMut<PlayerDead>) {
    health.0 = config.player_max_health;
    dead.0 = false;
}

fn main() -> ExitCode {
    let args = MpClientArgs::parse();

    let map_ron = fs::read_to_string(&args.map_path)
        .unwrap_or_else(|e| panic!("failed to read map {}: {}", args.map_path.display(), e));

    let sky_path = args.sky_path.to_string_lossy().to_string();

    let mut app = App::new();

    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "CARCINISATION MP".into(),
                    resolution: UVec2::new(SCREEN_W * 4, SCREEN_H * 4).into(),
                    ..default()
                }),
                ..default()
            })
            .set(AssetPlugin {
                file_path: "../../assets".into(),
                ..default()
            }),
    );

    app.add_plugins(CxPlugin::<Layer>::new(
        UVec2::new(SCREEN_W, SCREEN_H),
        "palette/base.png",
    ));

    app.insert_resource(Config {
        map_ron,
        sky_path,
        screen_width: SCREEN_W,
        screen_height: SCREEN_H,
        authority_mode: FpsAuthorityMode::RemoteClient,
        ..Default::default()
    });

    app.add_plugins(FpsPlugin::<Layer>::new());
    app.add_plugins(leafwing_input_manager::prelude::InputManagerPlugin::<
        carcinisation_input::GBInput,
    >::default());
    app.add_systems(Startup, carcinisation_input::init_gb_input);

    app.add_systems(Startup, |mut commands: Commands| {
        commands.spawn(Camera2d);
    });

    // Weapon HUD driven by replicated NetPlayer.current_attack via sync_weapon_hud_from_net_player.
    // Default starts as Flamethrower (index 0); server sets NetAttackId::None → syncs to Pistol (index 1).

    // God mode disabled — death/respawn is now server-authoritative.
    // To re-enable for testing: app.add_systems(Update, god_mode.after(Systems));

    if let Some(addr) = args.connect {
        let addr: SocketAddr = addr.parse().expect("invalid connect address");
        app.add_plugins(FpsClientPlugin { connect_addr: addr });
    }

    app.run();
    ExitCode::SUCCESS
}
