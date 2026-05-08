use bevy::prelude::*;
use bevy_replicon::prelude::*;

use crate::channels::{register_reliable_channels, register_unreliable_channels};
use crate::components::{
    NetEnemy, NetEnemyType, NetHealth, NetPickup, NetPlayer, NetProjectile, PlayerNetState,
};
use crate::protocol::{NetworkObjectId, Owner, PlayerId};
use crate::tick::{TickConfig, TickCounter, TickPlugin};

pub struct NetProtocolPlugin;

impl Plugin for NetProtocolPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TickPlugin);

        register_types(app);
    }
}

/// Register replicated components and network channels.
/// Must be called AFTER `RepliconPlugins` because it requires `ProtocolHasher`.
pub fn register_net_all(app: &mut App) {
    register_replication(app);
    register_reliable_channels(app);
    register_unreliable_channels(app);
}

fn register_types(app: &mut App) {
    app.register_type::<PlayerId>()
        .register_type::<NetworkObjectId>()
        .register_type::<Owner>()
        .register_type::<NetPlayer>()
        .register_type::<NetEnemy>()
        .register_type::<NetEnemyType>()
        .register_type::<NetProjectile>()
        .register_type::<NetPickup>()
        .register_type::<NetHealth>()
        .register_type::<PlayerNetState>()
        .register_type::<TickConfig>()
        .register_type::<TickCounter>();
}

fn register_replication(app: &mut App) {
    app.replicate::<NetPlayer>()
        .replicate::<NetEnemy>()
        .replicate::<NetProjectile>()
        .replicate::<NetPickup>()
        .replicate::<NetHealth>()
        .replicate::<PlayerId>()
        .replicate::<Owner>();
}
