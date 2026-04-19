//! Stage-scoped resources for tracking time, progress, and spawn timers.

use std::time::Duration;

use bevy::prelude::*;
use derive_new::new;

use super::{data::StageSpawn, projection::ProjectionProfile};

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
    pub spawns: Vec<StageSpawn>,
}

/// Resets all stage progression state to initial values.
/// Used by both stage restart and game-over-continue paths to ensure
/// consistent reset ordering.
pub fn reset_stage_progression(
    stage_progress: &mut StageProgress,
    stage_state: &mut NextState<super::StageProgressState>,
    stage_time: &mut Time<StageTimeDomain>,
    stage_action_timer: &mut StageActionTimer,
    start_index: usize,
) {
    stage_progress.index = start_index;
    stage_state.set(super::StageProgressState::Initial);
    *stage_time = Time::default();
    stage_action_timer.timer.reset();
    stage_action_timer.stop();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reset_stage_progression_uses_start_index() {
        let mut progress = StageProgress { index: 5 };
        let mut state = NextState::<super::super::StageProgressState>::default();
        let mut time = Time::<StageTimeDomain>::default();
        let mut timer = StageActionTimer::default();

        reset_stage_progression(&mut progress, &mut state, &mut time, &mut timer, 3);
        assert_eq!(progress.index, 3);
    }

    #[test]
    fn reset_stage_progression_zero_starts_fresh() {
        let mut progress = StageProgress { index: 7 };
        let mut state = NextState::<super::super::StageProgressState>::default();
        let mut time = Time::<StageTimeDomain>::default();
        let mut timer = StageActionTimer::default();

        reset_stage_progression(&mut progress, &mut state, &mut time, &mut timer, 0);
        assert_eq!(progress.index, 0);
    }
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
    #[must_use]
    pub fn new(acceleration: f32) -> Self {
        Self { acceleration }
    }

    /// Create standard gravity
    #[must_use]
    pub fn standard() -> Self {
        Self::new(Self::STANDARD)
    }

    /// Create low gravity
    #[must_use]
    pub fn low() -> Self {
        Self::new(Self::LOW)
    }

    /// Create zero gravity
    #[must_use]
    pub fn zero() -> Self {
        Self::new(Self::ZERO)
    }

    /// Create high gravity
    #[must_use]
    pub fn high() -> Self {
        Self::new(Self::HIGH)
    }
}

impl Default for StageGravity {
    fn default() -> Self {
        Self::standard()
    }
}

/// The currently effective stage projection profile.
///
/// Updated each frame from stage data + elapsed time so spawn and placement
/// systems can derive floor-relative positions without re-walking the step
/// timeline.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct ActiveProjection(pub ProjectionProfile);
