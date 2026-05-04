use bevy::prelude::*;
use carcinisation_net::NetProtocolPlugin;

/// Server plugin — wires protocol types, replication, and channels.
/// Transport (native UDP / WebSocket) added in Phase 2/4.
pub struct ServerPlugin;

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(NetProtocolPlugin);
    }
}
