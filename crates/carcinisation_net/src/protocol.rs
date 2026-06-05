use bevy::prelude::*;
use bevy_math::Vec2;
use serde::{Deserialize, Serialize};

use crate::components::NetProjectileType;
use crate::tick::InputSequence;

// Legacy button bitfield removed — superseded by ClientIntent + PlayerActions.

// ---- Connect mode -----------------------------------------------------------

/// Client connection mode, communicated via renet2 `user_data` during handshake.
///
/// Encoded into the 256-byte `user_data` field of `ClientAuthentication::Unsecure`.
/// The server reads this in `handle_client_connect` to decide whether to spawn a
/// player entity or treat the connection as a passive observer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ConnectMode {
    /// Standard player client — server spawns a `NetPlayer` entity.
    #[default]
    Player,
    /// Passive map monitor — server skips player spawn, client receives all replication.
    Monitor,
}

/// Size of the `renet2` netcode `user_data` field.
const USER_DATA_BYTES: usize = 256;

impl ConnectMode {
    /// Encode into the 256-byte `user_data` field for `ClientAuthentication`.
    #[must_use]
    pub const fn to_user_data(self) -> [u8; USER_DATA_BYTES] {
        let mut data = [0u8; USER_DATA_BYTES];
        data[0] = match self {
            Self::Player => 0,
            Self::Monitor => 1,
        };
        data
    }

    /// Decode from the 256-byte `user_data` field. Defaults to `Player` for
    /// unrecognised values (backward compatible with clients that send `None`).
    #[must_use]
    pub const fn from_user_data(data: &[u8; USER_DATA_BYTES]) -> Self {
        match data[0] {
            1 => Self::Monitor,
            _ => Self::Player,
        }
    }
}

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
    pub const fn has(self, flag: u8) -> bool {
        self.bits & flag != 0
    }

    pub const fn set(&mut self, flag: u8) {
        self.bits |= flag;
    }

    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.bits == 0
    }

    /// Create from raw bits, stripping undefined bits for safety.
    #[must_use]
    pub const fn from_raw(bits: u8) -> Self {
        Self {
            bits: bits & Self::VALID_MASK,
        }
    }

    #[must_use]
    pub const fn raw(self) -> u8 {
        self.bits
    }

    /// Merge another set of actions into this one (OR).
    pub const fn merge(&mut self, other: Self) {
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
    /// In `AimCommitment` mode, only respected when `aim_held` is true.
    pub fire_held: bool,
    /// Edge-triggered one-shot actions for this tick.
    pub actions: PlayerActions,
    /// Aim mode active. In `AimCommitment` this mirrors held B.
    /// In `Legacy` mode this field is ignored.
    pub aim_held: bool,
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
            aim_held: false,
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
            && !self.aim_held
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

/// Monitor acknowledgement — server → client (reliable).
/// Sent to monitor clients instead of `PlayerIdAssigned`. Confirms the
/// connection is active and replication will follow.
#[derive(Debug, Clone, Event, Serialize, Deserialize)]
pub struct MonitorAck;

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
///
/// Speed modifier state is included so client prediction replay uses the
/// same movement speed as the server, eliminating drift during web slow.
///
/// **Protocol versioning**: This struct is serialised with postcard
/// (non-self-describing, positional). Adding or removing fields is a
/// breaking change — client and server must be the same version.
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
    /// Total arc of the snap turn in radians (PI for 180°, PI/2 for 90°).
    pub snap_total_radians: f32,
    /// Server snap turn angular speed (rad/s).
    pub snap_speed: f32,
    /// Server snap turn direction (+1.0 left, -1.0 right).
    pub snap_direction: f32,
    /// Active speed modifier multiplier (0.1..=1.0), or 1.0 if none active.
    /// Used by client prediction replay for movement-speed parity.
    pub speed_modifier_multiplier: f32,
    /// Active speed modifier remaining drain budget.
    /// No modifier is active when `remaining <= 0.0` — this is the sentinel.
    /// A modifier with `multiplier = 1.0` and `remaining > 0.0` is valid but
    /// has no speed effect (still drains budget).
    pub speed_modifier_remaining: f32,
    /// Active push impulse direction (normalised or zero). Zero when no
    /// impulse is active. Used by client prediction replay for smooth
    /// multi-tick lunge push.
    pub impulse_direction_x: f32,
    pub impulse_direction_y: f32,
    /// Push impulse strength (map units/s). Zero when no impulse active.
    pub impulse_strength: f32,
    /// Push impulse remaining lifetime (seconds). No impulse when <= 0.
    pub impulse_remaining: f32,
    /// Push impulse total duration (seconds). Used for decay curve.
    pub impulse_duration: f32,
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
            aim_held: false,
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
            aim_held: false,
        };
        assert!(tiny.is_idle(), "tiny values should be idle");

        // Meaningful movement is not idle.
        let moving = ClientIntent {
            sequence: InputSequence(0),
            movement: Vec2::new(0.0, 0.5),
            turn: 0.0,
            fire_held: false,
            actions: PlayerActions::default(),
            aim_held: false,
        };
        assert!(!moving.is_idle(), "0.5 movement should not be idle");

        // Fire held is not idle.
        let firing = ClientIntent {
            sequence: InputSequence(0),
            movement: Vec2::ZERO,
            turn: 0.0,
            fire_held: true,
            actions: PlayerActions::default(),
            aim_held: false,
        };
        assert!(!firing.is_idle(), "fire held should not be idle");

        // Aim held is not idle — entering AimMode must be sent immediately
        // so the server can suppress translation and gate fire.
        let aiming = ClientIntent {
            sequence: InputSequence(0),
            movement: Vec2::ZERO,
            turn: 0.0,
            fire_held: false,
            actions: PlayerActions::default(),
            aim_held: true,
        };
        assert!(!aiming.is_idle(), "aim held should not be idle");
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
    fn hit_confirm_roundtrip() {
        let event = HitConfirm {
            target_id: NetworkObjectId(5),
            damage: 37.0,
            position: Vec2::new(3.5, 1.5),
            kind: HitImpactKind::Destroy,
            projectile_type: Some(NetProjectileType::BloodShot),
        };
        let back = roundtrip(&event);
        assert_eq!(back.target_id, NetworkObjectId(5));
        assert!((back.damage - 37.0).abs() < 1e-5);
        assert_eq!(back.kind, HitImpactKind::Destroy);
        assert_eq!(back.projectile_type, Some(NetProjectileType::BloodShot));
    }

    #[test]
    fn hit_confirm_roundtrip_no_projectile() {
        let event = HitConfirm {
            target_id: NetworkObjectId(5),
            damage: 37.0,
            position: Vec2::new(3.5, 1.5),
            kind: HitImpactKind::Hit,
            projectile_type: None,
        };
        let back = roundtrip(&event);
        assert_eq!(back.projectile_type, None);
    }

    #[test]
    fn damage_effect_roundtrip() {
        let event = DamageEffect {
            target_id: NetworkObjectId(10),
            damage: 25.0,
            remaining_health: 75.0,
            was_player: true,
        };
        let back = roundtrip(&event);
        assert_eq!(back.target_id, NetworkObjectId(10));
        assert!(back.was_player);
        assert!((back.remaining_health - 75.0).abs() < 1e-5);
    }

    #[test]
    fn death_effect_roundtrip() {
        let event = DeathEffect {
            target_id: NetworkObjectId(3),
            was_player: false,
        };
        let back = roundtrip(&event);
        assert_eq!(back.target_id, NetworkObjectId(3));
        assert!(!back.was_player);
    }

    #[test]
    fn flame_active_roundtrip() {
        let event = FlameActive {
            player_id: PlayerId(2),
            active: true,
        };
        let back = roundtrip(&event);
        assert_eq!(back.player_id, PlayerId(2));
        assert!(back.active);
    }

    #[test]
    fn flame_char_mark_roundtrip() {
        let event = FlameCharMark {
            cell_x: 5,
            cell_y: -3,
            side: 1,
            normal_sign: -1,
            u: 0.75,
            seed: 0xDEAD_BEEF,
        };
        let back = roundtrip(&event);
        assert_eq!(back.cell_x, 5);
        assert_eq!(back.cell_y, -3);
        assert_eq!(back.side, 1);
        assert_eq!(back.normal_sign, -1);
        assert!((back.u - 0.75).abs() < 1e-5);
        assert_eq!(back.seed, 0xDEAD_BEEF);
    }

    #[test]
    fn enemy_attack_visual_roundtrip() {
        for kind in [EnemyAttackKind::Ranged, EnemyAttackKind::Melee] {
            let event = EnemyAttackVisual {
                object_id: NetworkObjectId(42),
                kind,
            };
            let back = roundtrip(&event);
            assert_eq!(back.object_id, NetworkObjectId(42));
            assert_eq!(back.kind, kind);
        }
    }

    #[test]
    fn pickup_effect_roundtrip() {
        let event = PickupEffect {
            player_id: PlayerId(1),
            pickup_id: NetworkObjectId(99),
            kind: NetPickupKind::Ammo,
            position: Vec2::new(2.0, 3.0),
        };
        let back = roundtrip(&event);
        assert_eq!(back.player_id, PlayerId(1));
        assert_eq!(back.pickup_id, NetworkObjectId(99));
        assert_eq!(back.kind, NetPickupKind::Ammo);
    }

    #[test]
    fn player_id_assigned_roundtrip() {
        let event = PlayerIdAssigned(PlayerId(7));
        let back = roundtrip(&event);
        assert_eq!(back.0, PlayerId(7));
    }

    #[test]
    fn monitor_ack_roundtrip() {
        let event = MonitorAck;
        let _back: MonitorAck = roundtrip(&event);
    }

    #[test]
    fn connect_mode_user_data_roundtrip() {
        let player = ConnectMode::Player;
        assert_eq!(
            ConnectMode::from_user_data(&player.to_user_data()),
            ConnectMode::Player
        );

        let monitor = ConnectMode::Monitor;
        assert_eq!(
            ConnectMode::from_user_data(&monitor.to_user_data()),
            ConnectMode::Monitor
        );
    }

    #[test]
    fn connect_mode_from_zeroed_user_data_defaults_to_player() {
        let zeroed = [0u8; 256];
        assert_eq!(ConnectMode::from_user_data(&zeroed), ConnectMode::Player);
    }

    // -----------------------------------------------------------------------
    // Corrupted / malformed bytes — must not panic
    // -----------------------------------------------------------------------

    #[test]
    fn garbage_bytes_do_not_panic_client_intent() {
        let garbage = [0xFF, 0x00, 0xAB, 0xCD, 0xEF];
        let result = postcard::from_bytes::<ClientIntent>(&garbage);
        assert!(result.is_err());
    }

    #[test]
    fn empty_bytes_do_not_panic_client_intent() {
        let result = postcard::from_bytes::<ClientIntent>(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn truncated_bytes_do_not_panic_input_ack() {
        // Encode a valid InputAck, then truncate it.
        let ack = InputAck {
            player_id: PlayerId(1),
            last_processed_sequence: InputSequence(1),
            server_tick: crate::tick::Tick(1),
            position: Vec2::ZERO,
            angle: 0.0,
            snap_remaining_radians: 0.0,
            snap_total_radians: 0.0,
            snap_speed: 0.0,
            snap_direction: 0.0,
            speed_modifier_multiplier: 1.0,
            speed_modifier_remaining: 0.0,
            impulse_direction_x: 0.0,
            impulse_direction_y: 0.0,
            impulse_strength: 0.0,
            impulse_remaining: 0.0,
            impulse_duration: 0.0,
        };
        let bytes = postcard::to_allocvec(&ack).unwrap();
        // Try every truncation length — none should panic.
        for len in 0..bytes.len() {
            let _ = postcard::from_bytes::<InputAck>(&bytes[..len]);
        }
    }

    #[test]
    fn garbage_bytes_rejected_hit_confirm() {
        let garbage = [0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01, 0x02];
        assert!(postcard::from_bytes::<HitConfirm>(&garbage).is_err());
    }

    /// 32 bytes of 0xFF must be rejected by every protocol type.
    /// Postcard uses strict decoding — trailing bytes always error.
    #[test]
    fn garbage_bytes_rejected_all_event_types() {
        let garbage: &[u8] = &[0xFF; 32];
        assert!(postcard::from_bytes::<ClientIntent>(garbage).is_err());
        assert!(postcard::from_bytes::<InputAck>(garbage).is_err());
        assert!(postcard::from_bytes::<MuzzleFlash>(garbage).is_err());
        assert!(postcard::from_bytes::<HitConfirm>(garbage).is_err());
        assert!(postcard::from_bytes::<DamageEffect>(garbage).is_err());
        assert!(postcard::from_bytes::<DeathEffect>(garbage).is_err());
        assert!(postcard::from_bytes::<FlameActive>(garbage).is_err());
        assert!(postcard::from_bytes::<FlameCharMark>(garbage).is_err());
        assert!(postcard::from_bytes::<EnemyAttackVisual>(garbage).is_err());
        assert!(postcard::from_bytes::<PickupEffect>(garbage).is_err());
        assert!(postcard::from_bytes::<PlayerIdAssigned>(garbage).is_err());
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
            snap_total_radians: std::f32::consts::PI,
            snap_speed: 7.85,
            snap_direction: -1.0,
            speed_modifier_multiplier: 0.7,
            speed_modifier_remaining: 2.5,
            impulse_direction_x: -0.8,
            impulse_direction_y: 0.6,
            impulse_strength: 4.24,
            impulse_remaining: 0.15,
            impulse_duration: 0.25,
        };
        let back = roundtrip(&ack);
        assert_eq!(back.player_id.0, 7);
        assert_eq!(back.last_processed_sequence.0, 42);
        assert_eq!(back.server_tick.0, 100);
        assert!((back.position.x - 3.5).abs() < 1e-5);
        assert!((back.angle - 1.57).abs() < 1e-5);
        assert!((back.snap_remaining_radians - 1.2).abs() < 1e-5);
        assert!((back.snap_total_radians - std::f32::consts::PI).abs() < 1e-5);
        assert!((back.snap_speed - 7.85).abs() < 1e-5);
        assert!((back.snap_direction - -1.0).abs() < 1e-5);
        assert!((back.speed_modifier_multiplier - 0.7).abs() < 1e-5);
        assert!((back.speed_modifier_remaining - 2.5).abs() < 1e-5);
        assert!((back.impulse_direction_x - -0.8).abs() < 1e-5);
        assert!((back.impulse_direction_y - 0.6).abs() < 1e-5);
        assert!((back.impulse_strength - 4.24).abs() < 1e-5);
        assert!((back.impulse_remaining - 0.15).abs() < 1e-5);
        assert!((back.impulse_duration - 0.25).abs() < 1e-5);
    }
}
