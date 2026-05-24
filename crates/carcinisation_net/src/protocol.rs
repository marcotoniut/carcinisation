use bevy::prelude::*;
use bevy_math::Vec2;
use serde::{Deserialize, Serialize};

use crate::components::NetProjectileType;
use crate::tick::InputSequence;

// Legacy button bitfield removed — superseded by ClientIntent + PlayerActions.

// ---- Protocol types -------------------------------------------------------

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

// ---- Semantic intent protocol ------------------------------------------------

/// One-shot player actions, edge-triggered. Packed as a bitfield.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerActions {
    bits: u8,
}

impl PlayerActions {
    pub const SNAP_TURN_LEFT: u8 = 0x01;
    pub const SNAP_TURN_RIGHT: u8 = 0x02;
    pub const QUICK_TURN: u8 = 0x04;
    pub const WEAPON_SWITCH: u8 = 0x08;
    pub const MELEE: u8 = 0x10;

    /// Mask of all defined action bits. Undefined bits are stripped by `from_raw`.
    pub const VALID_MASK: u8 = Self::SNAP_TURN_LEFT
        | Self::SNAP_TURN_RIGHT
        | Self::QUICK_TURN
        | Self::WEAPON_SWITCH
        | Self::MELEE;

    #[must_use]
    pub fn has(self, flag: u8) -> bool {
        self.bits & flag != 0
    }

    pub fn set(&mut self, flag: u8) {
        self.bits |= flag;
    }

    #[must_use]
    pub fn is_empty(self) -> bool {
        self.bits == 0
    }

    /// Create from raw bits, stripping undefined bits for safety.
    #[must_use]
    pub fn from_raw(bits: u8) -> Self {
        Self {
            bits: bits & Self::VALID_MASK,
        }
    }

    #[must_use]
    pub fn raw(self) -> u8 {
        self.bits
    }

    /// Merge another set of actions into this one (OR).
    pub fn merge(&mut self, other: Self) {
        self.bits |= other.bits;
    }
}

/// Semantic intent sent from client → server (reliable, ordered).
///
/// The client resolves all chord detection, grace windows, and physical key
/// interpretation locally. The server receives pure gameplay intent.
#[derive(Debug, Clone, Event, Serialize, Deserialize)]
pub struct ClientIntent {
    pub sequence: InputSequence,
    /// Player-local movement intent: y = forward(+)/back(-), x = strafe right(+)/left(-).
    /// Normalized to unit length. Server scales by `move_speed` × dt.
    pub movement: Vec2,
    /// Continuous turn direction: +1.0 = left, -1.0 = right, 0.0 = none.
    /// Server scales by `turn_speed` × dt. Suppressed by server during snap turn animation.
    pub turn: f32,
    /// Fire button held. Server uses for pistol cooldown and flamethrower continuous fire.
    pub fire_held: bool,
    /// Edge-triggered one-shot actions for this tick.
    pub actions: PlayerActions,
}

impl ClientIntent {
    #[must_use]
    pub fn idle(sequence: InputSequence) -> Self {
        Self {
            sequence,
            movement: Vec2::ZERO,
            turn: 0.0,
            fire_held: false,
            actions: PlayerActions::default(),
        }
    }

    /// Epsilon for float comparisons in idle detection.
    const IDLE_EPSILON: f32 = 1e-5;

    #[must_use]
    pub fn is_idle(&self) -> bool {
        self.movement.length_squared() < Self::IDLE_EPSILON
            && self.turn.abs() < Self::IDLE_EPSILON
            && !self.fire_held
            && self.actions.is_empty()
    }
}

/// Muzzle flash effect — server → client (unreliable).
#[derive(Debug, Clone, Event, Serialize, Deserialize)]
pub struct MuzzleFlash {
    pub player_id: PlayerId,
    pub position: Vec2,
    pub angle: f32,
}

/// Projectile/hitscan impact — server → client (unreliable).
/// Carries the impact position so the client can render a billboard
/// without looking up the (possibly despawned) source entity.
#[derive(Debug, Clone, Event, Serialize, Deserialize)]
pub struct HitConfirm {
    pub target_id: NetworkObjectId,
    pub damage: f32,
    /// World-space position of the impact.
    pub position: Vec2,
    /// Visual kind: splat vs animated destroy.
    pub kind: HitImpactKind,
    /// Projectile type for projectile impacts; `None` for hitscan.
    /// Used by the client to select the correct destroy sprite.
    #[serde(default)]
    pub projectile_type: Option<NetProjectileType>,
}

/// Visual kind for impact billboards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HitImpactKind {
    /// Static blood splat (hitscan on enemy, projectile on player/wall).
    Hit,
    /// Animated destroy (projectile shot down by player).
    Destroy,
}

/// Damage effect — server → client (unreliable).
#[derive(Debug, Clone, Event, Serialize, Deserialize)]
pub struct DamageEffect {
    pub target_id: NetworkObjectId,
    pub damage: f32,
    pub remaining_health: f32,
    /// True when the target is a player, false for enemies.
    pub was_player: bool,
}

/// Death effect — server → client (unreliable).
#[derive(Debug, Clone, Event, Serialize, Deserialize)]
pub struct DeathEffect {
    pub target_id: NetworkObjectId,
    pub was_player: bool,
}

/// Flamethrower active state — server → client (unreliable).
/// Sent when a player starts or stops flamethrower fire.
#[derive(Debug, Clone, Event, Serialize, Deserialize)]
pub struct FlameActive {
    pub player_id: PlayerId,
    pub active: bool,
}

/// Flamethrower wall char mark — server → client (unreliable).
/// Emitted when a flamethrower ray hits a wall. Rate-limited per player.
#[derive(Debug, Clone, Event, Serialize, Deserialize)]
pub struct FlameCharMark {
    pub cell_x: i32,
    pub cell_y: i32,
    /// 0 = Vertical, 1 = Horizontal.
    pub side: u8,
    pub normal_sign: i8,
    /// Wall-face UV horizontal position (0.0–1.0).
    pub u: f32,
    /// Deterministic seed for decal shape/flip.
    pub seed: u32,
}

/// Enemy attack visual — server → client (unreliable).
/// Triggers a one-shot animation on the client for the attacking enemy.
#[derive(Debug, Clone, Event, Serialize, Deserialize)]
pub struct EnemyAttackVisual {
    pub object_id: NetworkObjectId,
    pub kind: EnemyAttackKind,
}

/// Kind of enemy attack for animation selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnemyAttackKind {
    Ranged,
    Melee,
}

/// Pickup effect — server → client (unreliable).
#[derive(Debug, Clone, Event, Serialize, Deserialize)]
pub struct PickupEffect {
    pub player_id: PlayerId,
    pub pickup_id: NetworkObjectId,
    pub kind: NetPickupKind,
    pub position: Vec2,
}

/// Player ID assignment — server → client (reliable).
/// Sent once when client connects so it knows which `NetPlayer` is "mine".
#[derive(Debug, Clone, Event, Serialize, Deserialize)]
pub struct PlayerIdAssigned(pub PlayerId);

/// Server acknowledgement of processed input — server → client (reliable, ordered).
///
/// Sent once per server tick for each player that has unacked input.
/// Carries the authoritative position/angle after applying the input,
/// enabling client-side prediction reconciliation.
///
/// Snap turn state is included so the client can continue an in-progress
/// snap turn after reconciliation prunes the history entry that initiated
/// it. Without this, the client zeroes snap state on every ack, causing
/// ~15° angle corrections per tick for the duration of the snap.
#[derive(Debug, Clone, Event, Serialize, Deserialize)]
pub struct InputAck {
    /// Which player this ack is for. Clients filter by their own `PlayerId`.
    pub player_id: PlayerId,
    /// Last `InputSequence` the server fully processed for this player.
    pub last_processed_sequence: InputSequence,
    /// Server tick at which this sequence was processed.
    pub server_tick: crate::tick::Tick,
    /// Authoritative position after movement processing.
    pub position: Vec2,
    /// Authoritative angle after movement processing.
    pub angle: f32,
    /// Server snap turn remaining radians (0.0 if no snap active).
    pub snap_remaining_radians: f32,
    /// Server snap turn angular speed (rad/s).
    pub snap_speed: f32,
    /// Server snap turn direction (+1.0 left, -1.0 right).
    pub snap_direction: f32,
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
        let bytes = postcard::to_allocvec(val).unwrap();
        postcard::from_bytes(&bytes).unwrap()
    }

    #[test]
    fn client_intent_roundtrip() {
        let intent = ClientIntent {
            sequence: InputSequence(42),
            movement: Vec2::new(0.0, 1.0),
            turn: -1.0,
            fire_held: true,
            actions: PlayerActions::from_raw(PlayerActions::WEAPON_SWITCH),
        };
        let back = roundtrip(&intent);
        assert_eq!(back.sequence.0, 42);
        assert!((back.movement.y - 1.0).abs() < 1e-5);
        assert!((back.turn - (-1.0)).abs() < 1e-5);
        assert!(back.fire_held);
        assert!(back.actions.has(PlayerActions::WEAPON_SWITCH));
    }

    #[test]
    fn player_actions_from_raw_strips_undefined_bits() {
        let all_bits = PlayerActions::from_raw(0xFF);
        assert_eq!(all_bits.raw(), PlayerActions::VALID_MASK);
        // Defined bits preserved.
        assert!(all_bits.has(PlayerActions::SNAP_TURN_LEFT));
        assert!(all_bits.has(PlayerActions::MELEE));
        // Undefined bit 0x80 stripped.
        assert_eq!(all_bits.raw() & 0x80, 0);
    }

    #[test]
    fn player_actions_defined_bits_preserved() {
        let actions =
            PlayerActions::from_raw(PlayerActions::SNAP_TURN_LEFT | PlayerActions::WEAPON_SWITCH);
        assert!(actions.has(PlayerActions::SNAP_TURN_LEFT));
        assert!(actions.has(PlayerActions::WEAPON_SWITCH));
        assert!(!actions.has(PlayerActions::MELEE));
    }

    #[test]
    fn client_intent_is_idle_epsilon() {
        // Exact zero is idle.
        assert!(ClientIntent::idle(InputSequence(0)).is_idle());

        // Tiny movement below epsilon is idle.
        let tiny = ClientIntent {
            sequence: InputSequence(0),
            movement: Vec2::new(1e-7, 1e-7),
            turn: 1e-7,
            fire_held: false,
            actions: PlayerActions::default(),
        };
        assert!(tiny.is_idle(), "tiny values should be idle");

        // Meaningful movement is not idle.
        let moving = ClientIntent {
            sequence: InputSequence(0),
            movement: Vec2::new(0.0, 0.5),
            turn: 0.0,
            fire_held: false,
            actions: PlayerActions::default(),
        };
        assert!(!moving.is_idle(), "0.5 movement should not be idle");

        // Fire held is not idle.
        let firing = ClientIntent {
            sequence: InputSequence(0),
            movement: Vec2::ZERO,
            turn: 0.0,
            fire_held: true,
            actions: PlayerActions::default(),
        };
        assert!(!firing.is_idle(), "fire held should not be idle");
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

    #[test]
    fn input_ack_roundtrip() {
        let ack = InputAck {
            player_id: PlayerId(7),
            last_processed_sequence: InputSequence(42),
            server_tick: crate::tick::Tick(100),
            position: Vec2::new(3.5, 7.2),
            angle: 1.57,
            snap_remaining_radians: 1.2,
            snap_speed: 7.85,
            snap_direction: -1.0,
        };
        let back = roundtrip(&ack);
        assert_eq!(back.player_id.0, 7);
        assert_eq!(back.last_processed_sequence.0, 42);
        assert_eq!(back.server_tick.0, 100);
        assert!((back.position.x - 3.5).abs() < 1e-5);
        assert!((back.angle - 1.57).abs() < 1e-5);
        assert!((back.snap_remaining_radians - 1.2).abs() < 1e-5);
        assert!((back.snap_speed - 7.85).abs() < 1e-5);
        assert!((back.snap_direction - -1.0).abs() < 1e-5);
    }
}
