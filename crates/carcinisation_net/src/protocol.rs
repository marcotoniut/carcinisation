use bevy::prelude::*;
use bevy_math::Vec2;
use serde::{Deserialize, Serialize};

use crate::tick::{InputSequence, Tick};

/// Stable player identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Component, Serialize, Deserialize, Reflect)]
#[reflect(Serialize, Deserialize)]
pub struct PlayerId(pub u32);

/// Stable ID for enemies, projectiles, pickups.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Component, Serialize, Deserialize, Reflect)]
#[reflect(Serialize, Deserialize)]
pub struct NetworkObjectId(pub u32);

/// Owning player for projectiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Component, Serialize, Deserialize, Reflect)]
#[reflect(Serialize, Deserialize)]
pub struct Owner(pub PlayerId);

/// Client input sent from client → server (reliable, ordered).
#[derive(Debug, Clone, Event, Serialize, Deserialize)]
pub struct ClientInput {
    pub sequence: InputSequence,
    pub tick: Tick,
    pub delta_time: f32,
    pub movement: Vec2,
    pub angle_delta: f32,
    pub buttons: u8,
}

/// Attack fire event — client → server (optional, richer payload).
#[derive(Debug, Clone, Event, Serialize, Deserialize)]
pub struct AttackFire {
    pub tick: Tick,
    pub attacker: PlayerId,
}

/// Muzzle flash effect — server → client (unreliable).
#[derive(Debug, Clone, Event, Serialize, Deserialize)]
pub struct MuzzleFlash {
    pub player_id: PlayerId,
    pub position: Vec2,
    pub angle: f32,
}

/// Hit confirmation — server → client (unreliable).
#[derive(Debug, Clone, Event, Serialize, Deserialize)]
pub struct HitConfirm {
    pub target_id: NetworkObjectId,
    pub damage: f32,
}

/// Damage effect — server → client (unreliable).
#[derive(Debug, Clone, Event, Serialize, Deserialize)]
pub struct DamageEffect {
    pub target_id: NetworkObjectId,
    pub damage: f32,
    pub remaining_health: f32,
}

/// Death effect — server → client (unreliable).
#[derive(Debug, Clone, Event, Serialize, Deserialize)]
pub struct DeathEffect {
    pub target_id: NetworkObjectId,
    pub was_player: bool,
}

/// Pickup effect — server → client (unreliable).
#[derive(Debug, Clone, Event, Serialize, Deserialize)]
pub struct PickupEffect {
    pub player_id: PlayerId,
    pub pickup_id: NetworkObjectId,
    pub kind: NetPickupKind,
}

/// Net-safe pickup kind enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
#[reflect(Serialize, Deserialize)]
pub enum NetPickupKind {
    Health,
    Ammo,
    Weapon,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip<T: Serialize + serde::de::DeserializeOwned>(val: &T) -> T {
        let bytes = bincode::serialize(val).unwrap();
        bincode::deserialize(&bytes).unwrap()
    }

    #[test]
    fn client_input_roundtrip() {
        let input = ClientInput {
            sequence: InputSequence(42),
            tick: Tick(100),
            delta_time: 0.033,
            movement: Vec2::new(0.5, -0.3),
            angle_delta: 0.1,
            buttons: 0b001,
        };
        let back = roundtrip(&input);
        assert_eq!(back.sequence.0, 42);
        assert_eq!(back.tick.0, 100);
        assert!((back.delta_time - 0.033).abs() < 1e-6);
        assert_eq!(back.buttons, 0b001);
    }

    #[test]
    fn attack_fire_roundtrip() {
        let event = AttackFire {
            tick: Tick(55),
            attacker: PlayerId(3),
        };
        let back = roundtrip(&event);
        assert_eq!(back.attacker.0, 3);
    }

    #[test]
    fn muzzle_flash_roundtrip() {
        let event = MuzzleFlash {
            player_id: PlayerId(1),
            position: Vec2::new(10.0, 20.0),
            angle: 1.57,
        };
        let back = roundtrip(&event);
        assert_eq!(back.player_id.0, 1);
    }

    #[test]
    fn pickup_kind_roundtrip() {
        let kind = NetPickupKind::Health;
        let back = roundtrip(&kind);
        assert_eq!(back, NetPickupKind::Health);
    }
}
