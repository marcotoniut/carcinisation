use bevy::prelude::*;
use carcinisation_server::ServerPlugin;

fn main() {
    App::new().add_plugins((MinimalPlugins, ServerPlugin)).run();
}
