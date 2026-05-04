use bevy::prelude::*;
use bevy_math::Vec2;
use serde::{Deserialize, Serialize};

use crate::protocol::{NetPickupKind, NetworkObjectId, Owner, PlayerId};
use crate::tick::Tick;

/// Net-safe enemy state enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
#[reflect(Serialize, Deserialize)]
pub enum NetEnemyState {
    Idle,
    Chase,
    Attack,
    Dead,
}

/// Net-safe attack ID enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Reflect)]
#[reflect(Serialize, Deserialize)]
pub enum NetAttackId {
    #[default]
    None,
    Melee,
    Projectile,
}

/// Player network state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
#[reflect(Serialize, Deserialize)]
pub enum PlayerNetState {
    Alive,
    Dead { respawn_timer: f32 },
}

/// Replicated player component.
#[derive(Component, Debug, Clone, Serialize, Deserialize, Reflect)]
#[reflect(Serialize, Deserialize)]
pub struct NetPlayer {
    pub player_id: PlayerId,
    pub position: Vec2,
    pub angle: f32,
    pub current_attack: NetAttackId,
    pub state: PlayerNetState,
}

/// Replicated enemy component.
#[derive(Component, Debug, Clone, Serialize, Deserialize, Reflect)]
#[reflect(Serialize, Deserialize)]
pub struct NetEnemy {
    pub object_id: NetworkObjectId,
    pub position: Vec2,
    pub angle: f32,
    pub state: NetEnemyState,
    pub enemy_type: u32,
}

/// Replicated projectile component.
#[derive(Component, Debug, Clone, Serialize, Deserialize, Reflect)]
#[reflect(Serialize, Deserialize)]
pub struct NetProjectile {
    pub object_id: NetworkObjectId,
    pub position: Vec2,
    pub angle: f32,
    pub ttl: f32,
    pub owner: Owner,
    pub damage: f32,
}

/// Replicated pickup component.
#[derive(Component, Debug, Clone, Serialize, Deserialize, Reflect)]
#[reflect(Serialize, Deserialize)]
pub struct NetPickup {
    pub object_id: NetworkObjectId,
    pub position: Vec2,
    pub kind: NetPickupKind,
    pub respawn_timer: Option<f32>,
}

/// Reusable health component for players and enemies.
#[derive(Component, Debug, Clone, Serialize, Deserialize, Reflect)]
#[reflect(Serialize, Deserialize)]
pub struct NetHealth {
    pub current: f32,
    pub max: f32,
}

/// Replicated tick resource — server's current tick.
#[derive(Resource, Debug, Clone, Deref, DerefMut, Serialize, Deserialize, Reflect)]
#[reflect(Serialize, Deserialize)]
pub struct ReplicatedTick(pub Tick);

impl Default for ReplicatedTick {
    fn default() -> Self {
        Self(Tick(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip_component<T: Component + Serialize + serde::de::DeserializeOwned>(val: &T) -> T {
        let bytes = bincode::serialize(val).unwrap();
        bincode::deserialize(&bytes).unwrap()
    }

    #[test]
    fn net_player_roundtrip() {
        let player = NetPlayer {
            player_id: PlayerId(1),
            position: Vec2::new(100.0, 200.0),
            angle: 1.57,
            current_attack: NetAttackId::Projectile,
            state: PlayerNetState::Alive,
        };
        let back = roundtrip_component(&player);
        assert_eq!(back.player_id.0, 1);
        assert!(matches!(back.state, PlayerNetState::Alive));
    }

    #[test]
    fn net_enemy_roundtrip() {
        let enemy = NetEnemy {
            object_id: NetworkObjectId(5),
            position: Vec2::new(3.0, 4.0),
            angle: 0.0,
            state: NetEnemyState::Chase,
            enemy_type: 0,
        };
        let back = roundtrip_component(&enemy);
        assert_eq!(back.object_id.0, 5);
        assert!(matches!(back.state, NetEnemyState::Chase));
    }

    #[test]
    fn net_health_roundtrip() {
        let health = NetHealth {
            current: 75.0,
            max: 100.0,
        };
        let back = roundtrip_component(&health);
        assert!((back.current - 75.0).abs() < 1e-6);
    }

    #[test]
    fn net_projectile_roundtrip() {
        let proj = NetProjectile {
            object_id: NetworkObjectId(10),
            position: Vec2::new(5.0, 5.0),
            angle: 0.78,
            ttl: 2.5,
            owner: Owner(PlayerId(2)),
            damage: 25.0,
        };
        let back = roundtrip_component(&proj);
        assert_eq!(back.owner.0.0, 2);
        assert!((back.ttl - 2.5).abs() < 1e-6);
    }

    #[test]
    fn net_pickup_roundtrip() {
        let pickup = NetPickup {
            object_id: NetworkObjectId(3),
            position: Vec2::new(7.0, 8.0),
            kind: NetPickupKind::Health,
            respawn_timer: Some(5.0),
        };
        let back = roundtrip_component(&pickup);
        assert_eq!(back.kind, NetPickupKind::Health);
        assert_eq!(back.respawn_timer, Some(5.0));
    }

    #[test]
    fn replicated_tick_defaults() {
        let tick = ReplicatedTick::default();
        assert_eq!(tick.0.0, 0);
    }
}
