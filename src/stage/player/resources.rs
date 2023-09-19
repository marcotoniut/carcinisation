use bevy::prelude::*;

#[derive(Resource)]
pub struct AttackTimer {
    pub timer: Timer,
}

impl Default for AttackTimer {
    fn default() -> Self {
        AttackTimer {
            timer: Timer::from_seconds(0.0, TimerMode::Once),
        }
    }
}
