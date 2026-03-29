//! Stage-scoped resources for tracking time, progress, and spawn timers.

use std::time::Duration;

use bevy::prelude::*;
use derive_new::new;

use super::data::StageSpawn;

#[derive(Resource, Default, Clone, Copy, Debug)]
/// Marker used to scope Bevy's `Time` to the active stage.
pub struct StageTimeDomain;

#[derive(Clone, Debug, Default, Resource)]
/// Stores the active stage step index.
pub struct StageProgress {
    pub index: usize,
}

#[derive(Resource)]
/// Wrapper timer used to pace scripted stage actions.
pub struct StageActionTimer {
    pub timer: Timer,
}

impl StageActionTimer {
    pub fn start(&mut self, duration: Duration) {
        self.timer.set_duration(duration);
        self.timer.reset();
        self.timer.unpause();
    }

    pub fn stop(&mut self) {
        self.timer.pause();
    }
}

impl Default for StageActionTimer {
    fn default() -> Self {
        let mut timer = Timer::from_seconds(0., TimerMode::Once);
        timer.pause();
        StageActionTimer { timer }
    }
}

#[derive(new, Component, Default)]
/// Component that sequences stage spawns and tracks elapsed times.
pub struct StageStepSpawner {
    #[new(default)]
    pub elapsed: Duration,
    #[new(default)]
    pub elapsed_since_spawn: Duration,
    pub spawns: Vec<StageSpawn>,
}

#[derive(Resource, Clone, Copy, Debug, Reflect)]
#[reflect(Resource)]
/// Stage-specific gravity configuration.
///
/// Different stages can have different gravitational forces:
/// - Standard stages: ~800.0 px/s² (Earth-like)
/// - Low gravity stages: ~300.0 px/s² (Moon-like)
/// - Zero gravity stages: 0.0 px/s² (Outer space)
/// - High gravity stages: ~1200.0 px/s² (Heavy planet)
///
/// This affects falling enemies (like mosquitons with broken wings),
/// projectiles with arc trajectories (like boulder throws), and any
/// other gravity-dependent mechanics.
pub struct StageGravity {
    /// Gravitational acceleration in pixels per second squared.
    /// Positive values indicate downward acceleration (Y increases upward in this coordinate system,
    /// so gravity is applied negatively to make things fall down).
    pub acceleration: f32,
}

impl StageGravity {
    /// Standard Earth-like gravity for most stages
    pub const STANDARD: f32 = 800.0;

    /// Low gravity for moon or low-G environments
    pub const LOW: f32 = 300.0;

    /// Zero gravity for outer space stages
    pub const ZERO: f32 = 0.0;

    /// High gravity for dense planets
    pub const HIGH: f32 = 1200.0;

    /// Create a new gravity configuration
    pub fn new(acceleration: f32) -> Self {
        Self { acceleration }
    }

    /// Create standard gravity
    pub fn standard() -> Self {
        Self::new(Self::STANDARD)
    }

    /// Create low gravity
    pub fn low() -> Self {
        Self::new(Self::LOW)
    }

    /// Create zero gravity
    pub fn zero() -> Self {
        Self::new(Self::ZERO)
    }

    /// Create high gravity
    pub fn high() -> Self {
        Self::new(Self::HIGH)
    }
}

impl Default for StageGravity {
    fn default() -> Self {
        Self::standard()
    }
}
