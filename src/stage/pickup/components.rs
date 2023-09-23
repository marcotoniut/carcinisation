use bevy::prelude::*;

#[derive(Component, Debug, Clone)]
pub struct HealthRecovery(pub u32);

impl HealthRecovery {
    pub fn score_deduction(&self) -> i32 {
        -(self.0 as i32) * 10
    }
}
