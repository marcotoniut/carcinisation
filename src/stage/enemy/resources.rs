use bevy::prelude::*;

#[derive(Component)]
pub struct EnemyBehaviorTimer {
    pub entity: Entity,
    pub timer: Timer,
}

impl EnemyBehaviorTimer {
    pub fn new(entity: Entity, duration: f32) -> Self {
        EnemyBehaviorTimer {
            entity,
            timer: Timer::from_seconds(duration, TimerMode::Once),
        }
    }

    pub fn finished(&self) -> bool {
        self.timer.finished()
    }
}

#[derive(Component)]
pub struct RangedAttackTimer {
    pub entity: Entity,
    pub timer: Timer,
}

impl RangedAttackTimer {
    pub fn new(entity: Entity, duration: f32) -> Self {
        RangedAttackTimer {
            entity,
            timer: Timer::from_seconds(duration, TimerMode::Repeating),
        }
    }

    pub fn finished(&self) -> bool {
        self.timer.finished()
    }
}
