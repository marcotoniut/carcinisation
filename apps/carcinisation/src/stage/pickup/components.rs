use bevy::prelude::*;
use seldom_pixel::prelude::PxFilter;
use std::time::Duration;

#[derive(Component, Debug, Clone, Reflect)]
pub struct HealthRecovery(pub u32);

impl HealthRecovery {
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
    pub glitter_filter: PxFilter,
    pub original_filter: Option<PxFilter>,
}

impl PickupFeedbackGlitter {
    pub fn new(
        start_at: Duration,
        end_at: Duration,
        toggle_interval: Duration,
        glitter_filter: PxFilter,
        original_filter: Option<PxFilter>,
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
