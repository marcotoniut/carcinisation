use bevy::prelude::*;

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
