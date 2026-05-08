pub mod combat;
pub mod enemy_ai;
pub mod enemy_attack;
pub mod input;
pub mod player_lifecycle;
pub mod projectile;

pub use carcinisation_net::components::NetEnemy;
pub use carcinisation_net::{NetEnemyState, NetEnemyType, NetHealth, NetProjectile};
pub use combat::{
    BurnContactCooldowns, FireCooldownMap, FlameActiveTracker, FlameCharCooldowns,
    tick_burn_contact_damage, tick_despawn_timers, tick_enemy_death_timers,
};
pub use enemy_ai::{EnemyAiSet, ServerEnemyAiConfig, tick_net_enemy_ai};
pub use enemy_attack::{
    EnemyAttackSet, NextProjectileId, ServerMosquitonSim, ServerMosquitonSimConfig,
    tick_enemy_attacks, tick_pending_projectiles,
};
pub use input::{
    PlayerInputTracker, PlayerIntentBuffer, ServerQuickTurn, ServerTurnConfig, SnapTurnKind,
};
pub use player_lifecycle::tick_player_lifecycle;
pub use projectile::{ProjectileSet, ProjectileTtl, tick_projectiles_server};
