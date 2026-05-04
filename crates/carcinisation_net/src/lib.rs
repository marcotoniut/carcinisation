// carcinisation_net: Shared protocol + replication types

pub mod channels;
pub mod components;
pub mod plugin;
pub mod protocol;
pub mod tick;

pub use plugin::{NetProtocolPlugin, NetTransportPlugin};

// Re-export key types
pub use components::{NetAttackId, NetEnemyState};
pub use components::{
    NetEnemy, NetHealth, NetPickup, NetPlayer, NetProjectile, PlayerNetState, ReplicatedTick,
};
pub use protocol::{
    AttackFire, ClientInput, DamageEffect, DeathEffect, HitConfirm, MuzzleFlash, NetPickupKind,
    NetworkObjectId, Owner, PickupEffect, PlayerId,
};
pub use tick::{InputSequence, Tick, TickConfig, TickCounter, TickPlugin, TickSet};

pub const PROTOCOL_ID: u64 = 0xC4AC1253D; // TEMP: local testing only, replace later
