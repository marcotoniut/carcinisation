use bevy::prelude::*;
use carapace::prelude::CxFilter;
use std::time::Duration;

/// Interpolates scale on a pickup feedback entity over time.
#[derive(Clone, Component, Debug)]
pub struct PickupFeedbackScale {
    pub start_scale: f32,
    pub end_scale: f32,
    pub start_at: Duration,
    pub end_at: Duration,
}

/// Drop physics applied to a pickup spawned from a dead enemy.
#[derive(Clone, Component, Debug)]
pub struct PickupDropPhysics {
    pub velocity_y: f32,
    pub gravity: f32,
    pub floor_y: f32,
}

impl PickupDropPhysics {
    /// Creates drop physics with an initial upward velocity.
    #[must_use]
    pub fn new(spawn_y: f32, floor_y: f32, gravity: f32) -> Self {
        Self {
            velocity_y: 60.0,
            gravity,
            floor_y: floor_y.max(16.0).min(spawn_y),
        }
    }
}

#[derive(Component, Debug, Clone, Reflect)]
pub struct HealthRecovery(pub u32);

impl HealthRecovery {
    #[must_use]
    pub fn score_deduction(&self) -> i32 {
        -(self.0 as i32) * 2
    }
}

#[derive(Clone, Component, Debug, Default)]
pub struct PickupFeedback;

pub const PICKUP_FEEDBACK_TIME: f32 = 0.6;
pub const PICKUP_FEEDBACK_INITIAL_SPEED_Y: f32 = 100.0;
pub const PICKUP_FEEDBACK_GLITTER_TIME: f32 = 0.2;
pub const PICKUP_FEEDBACK_GLITTER_TOGGLE_SECS: f32 = 0.05 / 1.75;
pub const PICKUP_HUD_GLITTER_TIME: f32 = 0.5;

#[derive(Clone, Component, Debug)]
pub struct PickupFeedbackGlitter {
    pub start_at: Duration,
    pub end_at: Duration,
    pub toggle_interval: Duration,
    pub next_toggle_at: Duration,
    pub filter_on: bool,
    pub glitter_filter: CxFilter,
    pub original_filter: Option<CxFilter>,
}

impl PickupFeedbackGlitter {
    #[must_use]
    pub fn new(
        start_at: Duration,
        end_at: Duration,
        toggle_interval: Duration,
        glitter_filter: CxFilter,
        original_filter: Option<CxFilter>,
    ) -> Self {
        Self {
            start_at,
            end_at,
            toggle_interval,
            next_toggle_at: start_at,
            filter_on: false,
            glitter_filter,
            original_filter,
        }
    }
}
