use bevy::prelude::*;
use serde::{Deserialize, Serialize};

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct InputSequence(pub u32);

impl InputSequence {
    pub fn increment(&mut self) {
        self.0 = self.0.wrapping_add(1);
    }
}

/// Tick configuration.
#[derive(Resource, Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct TickConfig {
    pub hz: u32,
    pub delta_secs: f32,
}

impl Default for TickConfig {
    fn default() -> Self {
        Self {
            hz: 30,
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

/// SystemSet for tick systems.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct TickSet;

/// Increment tick.
pub fn increment_tick(mut tick_counter: ResMut<TickCounter>) {
    tick_counter.0.increment();
}

/// Tick plugin.
pub struct TickPlugin;

impl Plugin for TickPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TickConfig::default())
            .insert_resource(TickCounter::default())
            .configure_sets(FixedUpdate, TickSet)
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
        assert_eq!(config.hz, 30);
        assert!((config.delta_secs - 1.0 / 30.0).abs() < 1e-6);
    }
}
