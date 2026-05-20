// carcinisation_net: Shared protocol + replication types
#![allow(clippy::used_underscore_binding)]

pub mod channels;
pub mod components;
pub mod plugin;
pub mod prediction;
pub mod protocol;
pub mod sim_hash;
pub mod tick;
pub mod transport;

pub use components::{NetAttackId, NetEnemyState, NetEnemyType, NetProjectileType};
pub use components::{
    NetBurning, NetEnemy, NetGroundFire, NetHealth, NetPickup, NetPlayer, NetProjectile,
    PlayerNetState,
};
pub use protocol::{
    // Semantic intent protocol
    ClientIntent,
    // Events
    DamageEffect,
    DeathEffect,
    EnemyAttackKind,
    EnemyAttackVisual,
    FlameActive,
    FlameCharMark,
    HitConfirm,
    HitImpactKind,
    InputAck,
    MuzzleFlash,
    NetPickupKind,
    NetworkObjectId,
    Owner,
    PickupEffect,
    PlayerActions,
    PlayerId,
    PlayerIdAssigned,
};
pub use tick::{
    CombatSet, InputSequence, MovementSet, Tick, TickConfig, TickCounter, TickPlugin, TickSet,
};

pub const PROTOCOL_ID: u64 = 0x000C_4AC1_253D;

pub use plugin::{NetProtocolPlugin, register_net_all};
