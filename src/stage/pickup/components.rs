use bevy::prelude::*;

#[derive(Component, Debug, Clone)]
pub struct HealthRecovery(pub u32);

impl HealthRecovery {
    pub fn score_deduction(&self) -> i32 {
        -(self.0 as i32) * 2
    }
}

#[derive(Component, Debug, Clone)]
pub struct PickupFeedback;

pub const PICKUP_FEEDBACK_TIME: f32 = 0.5;
pub const PICKUP_FEEDBACK_INITIAL_SPEED_Y: f32 = 100.0;
