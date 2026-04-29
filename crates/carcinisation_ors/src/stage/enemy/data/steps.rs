use bevy::prelude::*;
use cween::structs::TweenDirection;
use derive_more::From;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Reflect, Serialize)]
pub struct AttackEnemyStep {
    pub duration: f32,
}

impl AttackEnemyStep {
    #[must_use]
    pub fn base() -> Self {
        Self {
            duration: EnemyStep::max_duration(),
        }
    }

    #[must_use]
    pub fn with_duration(mut self, value: f32) -> Self {
        self.duration = value;
        self
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Reflect, Serialize)]
pub struct CircleAroundEnemyStep {
    pub depth_movement_o: Option<i8>,
    pub direction: TweenDirection,
    pub duration: Option<f32>,
    pub radius: Option<f32>,
}

impl CircleAroundEnemyStep {
    // TODO get rid of this
    #[must_use]
    pub fn base() -> Self {
        Self {
            depth_movement_o: None,
            direction: TweenDirection::Negative,
            duration: Some(EnemyStep::max_duration()),
            radius: Some(12.),
        }
    }

    #[must_use]
    pub fn opposite_direction(mut self) -> Self {
        self.direction = self.direction.opposite();
        self
    }

    #[must_use]
    pub fn depth_advance(mut self, value: u8) -> Self {
        self.depth_movement_o = Some(-(value as i8));
        self
    }

    #[must_use]
    pub fn without_depth_movement(mut self) -> Self {
        self.depth_movement_o = None;
        self
    }

    #[must_use]
    pub fn with_direction(mut self, value: TweenDirection) -> Self {
        self.direction = value;
        self
    }

    #[must_use]
    pub fn with_duration(mut self, value: f32) -> Self {
        self.duration = Some(value);
        self
    }

    #[must_use]
    pub fn with_radius(mut self, value: f32) -> Self {
        self.radius = Some(value);
        self
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Reflect, Serialize)]
pub struct IdleEnemyStep {
    pub duration: f32,
}

impl IdleEnemyStep {
    #[must_use]
    pub fn base() -> Self {
        Self {
            duration: EnemyStep::max_duration(),
        }
    }

    #[must_use]
    pub fn with_duration(mut self, value: f32) -> Self {
        self.duration = value;
        self
    }
}

impl Default for IdleEnemyStep {
    fn default() -> Self {
        Self::base()
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Reflect, Serialize)]
pub struct LinearTweenEnemyStep {
    pub depth_movement_o: Option<i8>,
    pub direction: Vec2,
    #[serde(default)]
    pub trayectory: f32,
}

impl LinearTweenEnemyStep {
    #[must_use]
    pub fn base() -> Self {
        Self {
            direction: Vec2::new(-1., 0.),
            depth_movement_o: None,
            trayectory: 0.,
        }
    }

    #[must_use]
    pub fn opposite_direction(mut self) -> Self {
        self.direction = Vec2::new(-self.direction.x, -self.direction.y);
        self
    }

    #[must_use]
    pub fn with_direction(mut self, x: f32, y: f32) -> Self {
        self.direction = Vec2::new(x, y);
        self
    }

    #[must_use]
    pub fn with_trayectory(mut self, value: f32) -> Self {
        self.trayectory = value;
        self
    }

    #[must_use]
    pub fn depth_advance(mut self, value: u8) -> Self {
        self.depth_movement_o = Some(-(value as i8));
        self
    }

    #[must_use]
    pub fn depth_retreat(mut self, value: u8) -> Self {
        self.depth_movement_o = Some(value as i8);
        self
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Reflect, Serialize)]
pub struct JumpEnemyStep {
    pub attacking: bool,
    pub coordinates: Vec2,
    pub depth_movement: Option<i8>,
    pub speed: f32,
}

impl JumpEnemyStep {
    #[must_use]
    pub fn base() -> Self {
        Self {
            coordinates: Vec2::ZERO,
            attacking: false,
            depth_movement: None,
            speed: 0.5,
        }
    }
}

// Should rename to EnemyBehavior?
#[derive(Clone, Copy, Debug, Deserialize, From, Reflect, Serialize)]
pub enum EnemyStep {
    Attack(AttackEnemyStep),
    Circle(CircleAroundEnemyStep),
    Idle(IdleEnemyStep),
    LinearTween(LinearTweenEnemyStep),
    Jump(JumpEnemyStep),
}

impl Default for EnemyStep {
    fn default() -> Self {
        IdleEnemyStep::default().into()
    }
}

impl EnemyStep {
    #[must_use]
    pub fn max_duration() -> f32 {
        99999.
    }

    #[must_use]
    pub fn get_duration(&self) -> f32 {
        self.get_duration_o().unwrap_or(EnemyStep::max_duration())
    }

    #[must_use]
    pub fn get_duration_o(&self) -> Option<f32> {
        match self {
            EnemyStep::Attack(AttackEnemyStep { duration, .. })
            | EnemyStep::Idle(IdleEnemyStep { duration, .. }) => Some(*duration),
            EnemyStep::Circle(CircleAroundEnemyStep { duration, .. }) => *duration,
            EnemyStep::LinearTween { .. } | EnemyStep::Jump { .. } => None,
        }
    }

    #[must_use]
    pub fn attack_base() -> AttackEnemyStep {
        AttackEnemyStep::base()
    }

    #[must_use]
    pub fn circle_around_base() -> CircleAroundEnemyStep {
        CircleAroundEnemyStep::base()
    }

    #[must_use]
    pub fn idle_base() -> IdleEnemyStep {
        IdleEnemyStep::base()
    }

    #[must_use]
    pub fn jump_base() -> JumpEnemyStep {
        JumpEnemyStep::base()
    }

    #[must_use]
    pub fn linear_movement_base() -> LinearTweenEnemyStep {
        LinearTweenEnemyStep::base()
    }
}
