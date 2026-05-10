use bevy::prelude::*;
use bevy_math::Vec2;
use serde::{Deserialize, Serialize};

use crate::protocol::{NetPickupKind, NetworkObjectId, Owner, PlayerId};

/// Net-safe enemy state enum.
///
/// Drives both gameplay decisions (server) and animation selection (client).
/// One-shot attack animations are driven by `EnemyAttackVisual` events, not
/// replicated state, to avoid stale one-shot states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
#[reflect(Serialize, Deserialize)]
pub enum NetEnemyState {
    Idle,
    Chase,
    /// Holding at preferred range, ready to attack. Client renders idle/wing loop.
    HoldingRange,
    /// Playing death animation. Client renders death or burn pose.
    Dying {
        burn: bool,
    },
    /// Fully dead, inert until despawn. `burn` preserves kill type for visuals.
    Dead {
        burn: bool,
    },
}

/// Net-safe enemy type enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
#[reflect(Serialize, Deserialize)]
pub enum NetEnemyType {
    Basic,
    Mosquiton,
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
    Dead,
}

/// Replicated player component.
#[derive(Component, Debug, Clone, Serialize, Deserialize, Reflect)]
#[reflect(Component, Serialize, Deserialize)]
pub struct NetPlayer {
    pub player_id: PlayerId,
    pub position: Vec2,
    pub angle: f32,
    pub current_attack: NetAttackId,
    pub state: PlayerNetState,
}

/// Replicated enemy component.
#[derive(Component, Debug, Clone, Serialize, Deserialize, Reflect)]
#[reflect(Component, Serialize, Deserialize)]
pub struct NetEnemy {
    pub object_id: NetworkObjectId,
    pub position: Vec2,
    pub angle: f32,
    pub state: NetEnemyState,
    pub enemy_type: NetEnemyType,
}

/// Net-safe projectile type enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Reflect)]
#[reflect(Serialize, Deserialize)]
pub enum NetProjectileType {
    #[default]
    BloodShot,
}

/// Replicated projectile component.
#[derive(Component, Debug, Clone, Serialize, Deserialize, Reflect)]
#[reflect(Component, Serialize, Deserialize)]
pub struct NetProjectile {
    pub object_id: NetworkObjectId,
    pub position: Vec2,
    pub angle: f32,
    pub owner: Owner,
    pub damage: f32,
    pub projectile_type: NetProjectileType,
}

/// Replicated pickup component.
#[derive(Component, Debug, Clone, Serialize, Deserialize, Reflect)]
#[reflect(Component, Serialize, Deserialize)]
pub struct NetPickup {
    pub object_id: NetworkObjectId,
    pub position: Vec2,
    pub kind: NetPickupKind,
    pub respawn_timer: Option<f32>,
}

/// Reusable health component for players and enemies.
#[derive(Component, Debug, Clone, Serialize, Deserialize, Reflect)]
#[reflect(Component, Serialize, Deserialize)]
pub struct NetHealth {
    pub current: f32,
    pub max: f32,
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
            enemy_type: NetEnemyType::Mosquiton,
        };
        let back = roundtrip_component(&enemy);
        assert_eq!(back.object_id.0, 5);
        assert!(matches!(back.state, NetEnemyState::Chase));
        assert_eq!(back.enemy_type, NetEnemyType::Mosquiton);
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
            owner: Owner(PlayerId(2)),
            damage: 25.0,
            projectile_type: NetProjectileType::BloodShot,
        };
        let back = roundtrip_component(&proj);
        assert_eq!(back.owner.0.0, 2);
        assert!((back.damage - 25.0).abs() < 1e-6);
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
}
