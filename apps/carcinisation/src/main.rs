//! Application entrypoint: wires Bevy runtime using the shared bootstrap.

use carcinisation::app::{AppLaunchOptions, build_app};

fn main() {
    let mut app = build_app(AppLaunchOptions::default());
    app.run();
}
