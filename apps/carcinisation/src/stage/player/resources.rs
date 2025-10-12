use bevy::prelude::*;

#[derive(Resource)]
pub struct AttackTimer {
    pub timer: Timer,
}

impl Default for AttackTimer {
    fn default() -> Self {
        let mut timer = Timer::from_seconds(0., TimerMode::Once);
        timer.pause();
        AttackTimer { timer }
    }
}
