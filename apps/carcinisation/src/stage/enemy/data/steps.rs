use bevy::prelude::*;
use cween::structs::TweenDirection;
use derive_more::From;
use serde::{Deserialize, Serialize};

#[cfg(feature = "derive-ts")]
use ts_rs::TS;

#[cfg_attr(feature = "derive-ts", derive(TS))]
#[cfg_attr(feature = "derive-ts", ts(export))]
#[derive(Clone, Copy, Debug, Deserialize, Reflect, Serialize)]
pub struct AttackEnemyStep {
    pub duration: f32,
}

impl AttackEnemyStep {
    pub fn base() -> Self {
        Self {
            duration: EnemyStep::max_duration(),
        }
    }

    pub fn with_duration(mut self, value: f32) -> Self {
        self.duration = value;
        self
    }
}

#[cfg_attr(feature = "derive-ts", derive(TS))]
#[cfg_attr(feature = "derive-ts", ts(export))]
#[derive(Clone, Copy, Debug, Deserialize, Reflect, Serialize)]
pub struct CircleAroundEnemyStep {
    pub depth_movement_o: Option<i8>,
    pub direction: TweenDirection,
    pub duration: Option<f32>,
    pub radius: Option<f32>,
}

impl CircleAroundEnemyStep {
    // TODO get rid of this
    pub fn base() -> Self {
        Self {
            depth_movement_o: None,
            direction: TweenDirection::Negative,
            duration: Some(EnemyStep::max_duration()),
            radius: Some(12.),
        }
    }

    pub fn opposite_direction(mut self) -> Self {
        self.direction = self.direction.opposite();
        self
    }

    pub fn depth_advance(mut self, value: u8) -> Self {
        self.depth_movement_o = Some(-(value as i8));
        self
    }

    pub fn without_depth_movement(mut self) -> Self {
        self.depth_movement_o = None;
        self
    }

    pub fn with_direction(mut self, value: TweenDirection) -> Self {
        self.direction = value;
        self
    }

    pub fn with_duration(mut self, value: f32) -> Self {
        self.duration = Some(value);
        self
    }

    pub fn with_radius(mut self, value: f32) -> Self {
        self.radius = Some(value);
        self
    }
}

#[cfg_attr(feature = "derive-ts", derive(TS))]
#[cfg_attr(feature = "derive-ts", ts(export))]
#[derive(Clone, Copy, Debug, Deserialize, Reflect, Serialize)]
pub struct IdleEnemyStep {
    pub duration: f32,
}

impl IdleEnemyStep {
    pub fn base() -> Self {
        Self {
            duration: EnemyStep::max_duration(),
        }
    }

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

#[cfg_attr(feature = "derive-ts", derive(TS))]
#[cfg_attr(feature = "derive-ts", ts(export))]
#[derive(Clone, Copy, Debug, Deserialize, Reflect, Serialize)]
pub struct LinearTweenEnemyStep {
    pub depth_movement_o: Option<i8>,
    #[cfg_attr(feature = "derive-ts", ts(type = "[number, number]"))]
    pub direction: Vec2,
    #[serde(default)]
    pub trayectory: f32,
}

impl LinearTweenEnemyStep {
    pub fn base() -> Self {
        Self {
            direction: Vec2::new(-1., 0.),
            depth_movement_o: None,
            trayectory: 0.,
        }
    }

    pub fn opposite_direction(mut self) -> Self {
        self.direction = Vec2::new(-self.direction.x, -self.direction.y);
        self
    }

    pub fn with_direction(mut self, x: f32, y: f32) -> Self {
        self.direction = Vec2::new(x, y);
        self
    }

    pub fn with_trayectory(mut self, value: f32) -> Self {
        self.trayectory = value;
        self
    }

    pub fn depth_advance(mut self, value: u8) -> Self {
        self.depth_movement_o = Some(-(value as i8));
        self
    }

    pub fn depth_retreat(mut self, value: u8) -> Self {
        self.depth_movement_o = Some(value as i8);
        self
    }
}

#[cfg_attr(feature = "derive-ts", derive(TS))]
#[cfg_attr(feature = "derive-ts", ts(export))]
#[derive(Clone, Copy, Debug, Deserialize, Reflect, Serialize)]
pub struct JumpEnemyStep {
    pub attacking: bool,
    #[cfg_attr(feature = "derive-ts", ts(type = "[number, number]"))]
    pub coordinates: Vec2,
    pub depth_movement: Option<i8>,
    pub speed: f32,
}

impl JumpEnemyStep {
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
#[cfg_attr(feature = "derive-ts", derive(TS))]
#[cfg_attr(feature = "derive-ts", ts(export))]
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
    pub fn max_duration() -> f32 {
        99999.
    }

    pub fn get_duration(&self) -> f32 {
        self.get_duration_o().unwrap_or(EnemyStep::max_duration())
    }

    pub fn get_duration_o(&self) -> Option<f32> {
        match self {
            EnemyStep::Attack(AttackEnemyStep { duration, .. }) => Some(*duration),
            EnemyStep::Circle(CircleAroundEnemyStep { duration, .. }) => *duration,
            EnemyStep::Idle(IdleEnemyStep { duration, .. }) => Some(*duration),
            EnemyStep::LinearTween { .. } => None,
            EnemyStep::Jump { .. } => None,
        }
    }

    pub fn attack_base() -> AttackEnemyStep {
        AttackEnemyStep::base()
    }

    pub fn circle_around_base() -> CircleAroundEnemyStep {
        CircleAroundEnemyStep::base()
    }

    pub fn idle_base() -> IdleEnemyStep {
        IdleEnemyStep::base()
    }

    pub fn jump_base() -> JumpEnemyStep {
        JumpEnemyStep::base()
    }

    pub fn linear_movement_base() -> LinearTweenEnemyStep {
        LinearTweenEnemyStep::base()
    }
}
