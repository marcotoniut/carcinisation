use bevy::prelude::*;

use crate::stage::components::Health;

#[derive(Component, Debug, Clone)]
pub struct HealthRecovery(pub u32);

impl HealthRecovery {
    pub fn score_deduction(&self) -> u32 {
        self.0 * 10
    }
}
