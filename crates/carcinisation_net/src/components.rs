use bevy::prelude::*;
use bevy_math::Vec2;
use serde::{Deserialize, Serialize};

use crate::protocol::{NetPickupKind, NetworkObjectId, Owner, PlayerId};

/// Net-safe enemy state enum.
///
/// Drives both gameplay decisions (server) and animation selection (client).
/// One-shot attack animations are driven by `EnemyAttackVisual` events, not
/// replicated state, to avoid stale one-shot states. Trade-off: late-joining
/// clients see `HoldingRange` for mid-attack enemies until the next event.
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
    Spidey,
}

/// Server-assigned palette permutation for avatar colour variation.
///
/// Six curated permutations of three colour groups (A, B, C).
/// The label convention groups indices by result order:
///   Abc = A stays A, B stays B, C stays C (identity)
///   Acb = A stays A, B↔C swap
///   Bac = B→A, A→B, C stays C (A↔B swap)
///   Bca = B→A, C→B, A→C (cycle)
///   Cab = C→A, A→B, B→C (cycle)
///   Cba = C→A, B→B, A→C (A↔C swap)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Reflect)]
#[reflect(Serialize, Deserialize)]
pub enum AvatarPaletteVariant {
    #[default]
    Abc,
    Acb,
    Bac,
    Bca,
    Cab,
    Cba,
}

impl AvatarPaletteVariant {
    pub const COUNT: usize = 6;
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
    /// Authoritative flamethrower fire state. Replicated on transition only.
    /// Clients use this for reconciliation if `FlameActive` events are dropped.
    pub flame_active: bool,
    /// Server-assigned palette variant for third-person billboard colour
    /// differentiation. `None` falls back to identity remap.
    #[serde(default)]
    pub avatar_palette_variant: Option<AvatarPaletteVariant>,
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
    /// Server-computed visual height offset (hop/leap arc).
    /// Presentation-only: used by clients for billboard positioning and hop
    /// animation detection. Defaults to 0.0 for backward compatibility.
    #[serde(default)]
    pub visual_height: f32,
    /// Presentation-only normalized animation phase for enemies whose visual
    /// height is non-monotonic, such as Spidey hops.
    #[serde(default)]
    pub visual_phase: f32,
}

/// Replicated temporary player speed modifier.
#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize, Reflect, PartialEq)]
#[reflect(Component, Serialize, Deserialize)]
pub struct NetSpeedModifier {
    /// Multiplier applied to base movement speed.
    pub multiplier: f32,
    /// Remaining drain budget. Server authoritative.
    pub remaining: f32,
}

/// Net-safe projectile type enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Reflect)]
#[reflect(Serialize, Deserialize)]
pub enum NetProjectileType {
    #[default]
    BloodShot,
    WebShot,
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

/// Replicated ground fire hazard spawned when an enemy dies from burning.
#[derive(Component, Debug, Clone, Serialize, Deserialize, Reflect)]
#[reflect(Component, Serialize, Deserialize)]
pub struct NetGroundFire {
    pub position: Vec2,
    /// Deterministic seed for visual flame placement.
    pub seed: u32,
}

/// Reusable health component for players and enemies.
#[derive(Component, Debug, Clone, Serialize, Deserialize, Reflect)]
#[reflect(Component, Serialize, Deserialize)]
pub struct NetHealth {
    pub current: f32,
    pub max: f32,
}

/// Replicated burn intensity for progressive fire damage.
///
/// Server-authoritative: the server owns `BurnState` and syncs intensity here.
/// Clients read this for visual flame rendering on burning entities.
#[derive(Component, Debug, Clone, Default, Serialize, Deserialize, Reflect)]
#[reflect(Component, Serialize, Deserialize)]
pub struct NetBurning {
    pub intensity: f32,
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
            flame_active: false,
            avatar_palette_variant: None,
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
            visual_height: 0.0,
            visual_phase: 0.0,
        };
        let back = roundtrip_component(&enemy);
        assert_eq!(back.object_id.0, 5);
        assert!(matches!(back.state, NetEnemyState::Chase));
        assert_eq!(back.enemy_type, NetEnemyType::Mosquiton);
    }

    #[test]
    fn net_enemy_spidey_roundtrip() {
        let enemy = NetEnemy {
            object_id: NetworkObjectId(7),
            position: Vec2::new(5.0, 6.0),
            angle: 1.0,
            state: NetEnemyState::HoldingRange,
            enemy_type: NetEnemyType::Spidey,
            visual_height: 0.15,
            visual_phase: 0.5,
        };
        let back = roundtrip_component(&enemy);
        assert_eq!(back.object_id.0, 7);
        assert!(matches!(back.state, NetEnemyState::HoldingRange));
        assert_eq!(back.enemy_type, NetEnemyType::Spidey);
    }

    #[test]
    fn net_projectile_webshot_roundtrip() {
        let proj = NetProjectile {
            object_id: NetworkObjectId(11),
            position: Vec2::new(3.0, 4.0),
            angle: 0.5,
            owner: Owner(PlayerId(1)),
            damage: 10.0,
            projectile_type: NetProjectileType::WebShot,
        };
        let back = roundtrip_component(&proj);
        assert_eq!(back.owner.0.0, 1);
        assert_eq!(back.projectile_type, NetProjectileType::WebShot);
    }

    #[test]
    fn net_speed_modifier_roundtrip() {
        let modifier = NetSpeedModifier {
            multiplier: 0.7,
            remaining: 2.5,
        };
        let back = roundtrip_component(&modifier);
        assert!((back.multiplier - 0.7).abs() < f32::EPSILON);
        assert!((back.remaining - 2.5).abs() < f32::EPSILON);
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
    fn avatar_palette_variant_roundtrip() {
        for variant in &[
            AvatarPaletteVariant::Abc,
            AvatarPaletteVariant::Acb,
            AvatarPaletteVariant::Bac,
            AvatarPaletteVariant::Bca,
            AvatarPaletteVariant::Cab,
            AvatarPaletteVariant::Cba,
        ] {
            let bytes = bincode::serialize(variant).unwrap();
            let back: AvatarPaletteVariant = bincode::deserialize(&bytes).unwrap();
            assert_eq!(*variant, back);
        }
    }

    #[test]
    fn net_player_roundtrip_with_avatar_variant() {
        let player = NetPlayer {
            player_id: PlayerId(1),
            position: Vec2::new(100.0, 200.0),
            angle: 1.57,
            current_attack: NetAttackId::Projectile,
            state: PlayerNetState::Alive,
            flame_active: false,
            avatar_palette_variant: Some(AvatarPaletteVariant::Bca),
        };
        let back = roundtrip_component(&player);
        assert_eq!(back.player_id.0, 1);
        assert_eq!(back.avatar_palette_variant, Some(AvatarPaletteVariant::Bca));
    }

    #[test]
    fn net_player_default_avatar_variant_is_none() {
        // The default-deserialized field should be None (missing on old servers).
        let bytes = bincode::serialize(&NetPlayer {
            player_id: PlayerId(1),
            position: Vec2::ZERO,
            angle: 0.0,
            current_attack: NetAttackId::None,
            state: PlayerNetState::Alive,
            flame_active: false,
            avatar_palette_variant: None,
        })
        .unwrap();
        let back: NetPlayer = bincode::deserialize(&bytes).unwrap();
        assert_eq!(back.avatar_palette_variant, None);
    }

    #[test]
    fn net_ground_fire_roundtrip() {
        let fire = NetGroundFire {
            position: Vec2::new(3.0, 4.0),
            seed: 12345,
        };
        let back = roundtrip_component(&fire);
        assert!((back.position.x - 3.0).abs() < 1e-6);
        assert_eq!(back.seed, 12345);
    }

    #[test]
    fn net_burning_roundtrip() {
        let burning = NetBurning { intensity: 0.75 };
        let back = roundtrip_component(&burning);
        assert!((back.intensity - 0.75).abs() < 1e-6);
    }

    #[test]
    fn net_burning_default_is_zero() {
        let burning = NetBurning::default();
        assert!((burning.intensity - 0.0).abs() < 1e-6);
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
