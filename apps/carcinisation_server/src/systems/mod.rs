pub mod admin;
pub mod combat;
pub mod diagnostics;
pub mod enemy_ai;
pub mod enemy_attack;
pub mod input;
pub mod occupancy;
pub mod pickup;
pub mod player_lifecycle;
pub mod projectile;
pub mod reset;

pub use carcinisation_fps_core::pickup::PickupRules;
pub use carcinisation_net::components::NetEnemy;
pub use carcinisation_net::{NetEnemyState, NetEnemyType, NetHealth, NetProjectile};
pub use combat::{
    BurnContactCooldowns, FireCooldownMap, FlameActiveTracker, FlameCharCooldowns,
    GroundFireContactCooldowns, GroundFireCount, tick_burn_contact_damage, tick_despawn_timers,
    tick_enemy_death_timers, tick_ground_fire_damage,
};
pub use enemy_ai::{EnemyAiSet, ServerEnemyAiConfig, tick_net_enemy_ai};
pub use enemy_attack::{
    EnemyAttackSet, NextProjectileId, ServerMosquitonSim, ServerMosquitonSimConfig,
    ServerSpideySim, ServerSpideySimConfig, tick_enemy_attacks, tick_pending_projectiles,
    tick_spidey_attacks,
};
pub use input::{PlayerInputTracker, PlayerIntentBuffer, ServerQuickTurn, send_input_acks};
pub use occupancy::{OccupancySet, OccupiesSpace, ServerPlayerImpulse};
pub use pickup::pickup_system;
pub use player_lifecycle::{RespawnTimer, tick_player_lifecycle};
pub use projectile::{ProjectileSet, ProjectileTtl, tick_projectiles_server};
