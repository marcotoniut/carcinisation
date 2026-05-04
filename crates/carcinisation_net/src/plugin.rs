use bevy::prelude::*;
use bevy_replicon::prelude::*;

use crate::channels::{register_reliable_channels, register_unreliable_channels};
use crate::components::{
    NetEnemy, NetHealth, NetPickup, NetPlayer, NetProjectile, PlayerNetState, ReplicatedTick,
};
use crate::protocol::{NetworkObjectId, Owner, PlayerId};
use crate::tick::{TickConfig, TickCounter, TickPlugin};

/// NetProtocolPlugin: registers all protocol types, replication, channels, and events.
/// Does NOT configure transport — that is handled by NetTransportPlugin (Phase 2/4).
pub struct NetProtocolPlugin;

impl Plugin for NetProtocolPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TickPlugin)
            .insert_resource(ReplicatedTick::default());

        register_types(app);
        register_replication(app);
        register_reliable_channels(app);
        register_unreliable_channels(app);
    }
}

/// Register all protocol types for reflection.
fn register_types(app: &mut App) {
    app.register_type::<PlayerId>()
        .register_type::<NetworkObjectId>()
        .register_type::<Owner>()
        .register_type::<NetPlayer>()
        .register_type::<NetEnemy>()
        .register_type::<NetProjectile>()
        .register_type::<NetPickup>()
        .register_type::<NetHealth>()
        .register_type::<PlayerNetState>()
        .register_type::<TickConfig>()
        .register_type::<TickCounter>()
        .register_type::<ReplicatedTick>();
}

/// Register replicated components with Replicon.
fn register_replication(app: &mut App) {
    app.replicate::<NetPlayer>()
        .replicate::<NetEnemy>()
        .replicate::<NetProjectile>()
        .replicate::<NetPickup>()
        .replicate::<NetHealth>()
        .replicate::<PlayerId>()
        .replicate::<Owner>();
}

/// NetTransportPlugin: native/websocket transport setup.
/// Stub for Phase 2 (native) and Phase 4 (WebSocket).
pub struct NetTransportPlugin {
    pub is_server: bool,
    pub port: u16,
}

impl Plugin for NetTransportPlugin {
    fn build(&self, _app: &mut App) {
        // Transport setup goes here in Phase 2/4
    }
}
