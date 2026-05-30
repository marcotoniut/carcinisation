//! Application entrypoint: wires Bevy runtime using the shared bootstrap.

use carcinisation::app::{AppLaunchOptions, build_app};
use carcinisation::first_person::FpsClientPlugin;
use clap::Parser;
use std::net::SocketAddr;

#[derive(Parser, Clone, Debug)]
struct Args {
    #[arg(long)]
    connect: Option<String>,
}

fn main() {
    let args = Args::parse();
    let options = AppLaunchOptions::default();

    let mut app = build_app(options);

    if let Some(connect_str) = args.connect {
        let addr: SocketAddr = connect_str.parse().expect("invalid connect address");
        app.add_plugins(FpsClientPlugin { connect_addr: addr });
    }

    app.run();
}
