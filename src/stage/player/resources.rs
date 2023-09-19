use bevy::prelude::*;

#[derive(Resource)]
pub struct AttackTimer {
    pub timer: Timer,
}

impl Default for AttackTimer {
    fn default() -> Self {
        AttackTimer {
            // HACK to avoid triggering it the first time
            timer: Timer::from_seconds(99999.0, TimerMode::Once),
        }
    }
}
