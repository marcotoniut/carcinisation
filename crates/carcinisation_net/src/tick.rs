use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;

/// Wrapped tick counter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct Tick(pub u32);

impl Tick {
    pub fn increment(&mut self) {
        self.0 = self.0.wrapping_add(1);
    }
}

impl std::fmt::Display for Tick {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Tick({})", self.0)
    }
}

/// Input sequence counter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, Default)]
pub struct InputSequence(pub u32);

impl InputSequence {
    pub fn increment(&mut self) {
        self.0 = self.0.wrapping_add(1);
    }

    /// Wrapping-aware comparison (RFC 1982 serial number arithmetic).
    /// Returns `true` if `self` is strictly after `other` in the sequence space.
    #[must_use]
    pub fn is_after(self, other: Self) -> bool {
        let diff = self.0.wrapping_sub(other.0);
        diff != 0 && diff < (u32::MAX / 2)
    }
}

/// Tick configuration.
#[derive(Resource, Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct TickConfig {
    pub hz: NonZeroU32,
    pub delta_secs: f32,
}

impl Default for TickConfig {
    fn default() -> Self {
        Self {
            hz: NonZeroU32::new(30).unwrap(),
            delta_secs: 1.0 / 30.0,
        }
    }
}

/// Tick counter resource.
#[derive(Resource, Debug, Clone, Deref, DerefMut, Serialize, Deserialize, Reflect)]
pub struct TickCounter(pub Tick);

impl Default for TickCounter {
    fn default() -> Self {
        Self(Tick(0))
    }
}

/// `SystemSet` for server-authoritative movement, runs first in `FixedUpdate`.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct MovementSet;

/// `SystemSet` for server-authoritative combat (hitscan, damage), runs after movement.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct CombatSet;

/// `SystemSet` for tick systems, runs last in `FixedUpdate`.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct TickSet;

/// How many `FixedUpdate` ticks the server reapplies stale intent data
/// before zeroing it. At 30 Hz this is ~167 ms — enough to tolerate
/// one dropped packet. The client must use the same value for
/// stale-tick prediction parity.
pub const STALE_INPUT_TICKS: u32 = 5;

/// Increment tick.
pub fn increment_tick(mut tick_counter: ResMut<TickCounter>) {
    tick_counter.0.increment();
}

/// Tick plugin — shared by all consumers (server, client, tests).
///
/// Configures `FixedUpdate` at 30 Hz and establishes the **base** set ordering:
///
///   `MovementSet` → `CombatSet` → `TickSet`
///
/// The server extends this chain with intermediate sets (e.g.
/// `EnemyAiSet`, `EnemyAttackSet`, `ProjectileSet`) between `MovementSet`
/// and `CombatSet`. Bevy merges ordering constraints additively, so the
/// extended chain refines this base without contradicting it.
pub struct TickPlugin;

impl Plugin for TickPlugin {
    fn build(&self, app: &mut App) {
        let tick_config = TickConfig::default();

        // Align Bevy's FixedUpdate timestep with our tick rate.
        // Time<Fixed> is initialized by Bevy's TimePlugin (included in MinimalPlugins).
        if app.world().contains_resource::<Time<Fixed>>() {
            app.world_mut()
                .resource_mut::<Time<Fixed>>()
                .set_timestep_hz(f64::from(tick_config.hz.get()));
        }

        app.insert_resource(tick_config)
            .insert_resource(TickCounter::default())
            .configure_sets(FixedUpdate, (MovementSet, CombatSet, TickSet).chain())
            .add_systems(FixedUpdate, increment_tick.in_set(TickSet));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_wraps() {
        let mut tick = Tick(u32::MAX);
        tick.increment();
        assert_eq!(tick.0, 0);
    }

    #[test]
    fn tick_config_default() {
        let config = TickConfig::default();
        assert_eq!(config.hz.get(), 30);
        assert!((config.delta_secs - 1.0 / 30.0).abs() < 1e-6);
    }

    // ── InputSequence::is_after edge cases ──────────────────────────

    #[test]
    fn is_after_simple() {
        assert!(InputSequence(2).is_after(InputSequence(1)));
        assert!(!InputSequence(1).is_after(InputSequence(2)));
    }

    #[test]
    fn is_after_equal_is_false() {
        assert!(!InputSequence(5).is_after(InputSequence(5)));
    }

    #[test]
    fn is_after_zero_after_max() {
        // 0 wraps around and is "after" u32::MAX.
        assert!(InputSequence(0).is_after(InputSequence(u32::MAX)));
    }

    #[test]
    fn is_after_max_not_after_zero() {
        // u32::MAX is NOT after 0 (it's far behind in wrapping space).
        assert!(!InputSequence(u32::MAX).is_after(InputSequence(0)));
    }

    #[test]
    fn is_after_wrapping_small_gap() {
        // 3 is after u32::MAX - 2 (wrapping by 5).
        assert!(InputSequence(3).is_after(InputSequence(u32::MAX - 2)));
    }

    #[test]
    fn is_after_half_range_boundary() {
        // At exactly half the u32 range, is_after returns false (ambiguous).
        let half = u32::MAX / 2;
        assert!(!InputSequence(half).is_after(InputSequence(0)));
    }
}
