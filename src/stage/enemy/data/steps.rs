use crate::plugins::movement::structs::MovementDirection;
use bevy::prelude::*;

#[derive(Clone, Copy, Debug)]
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

#[derive(Clone, Copy, Debug)]
pub struct CircleAroundEnemyStep {
    pub depth_movement_o: Option<i8>,
    pub direction: MovementDirection,
    pub duration: f32,
    pub radius: f32,
}

impl CircleAroundEnemyStep {
    pub fn base() -> Self {
        Self {
            depth_movement_o: None,
            direction: MovementDirection::Negative,
            duration: EnemyStep::max_duration(),
            radius: 12.,
        }
    }

    pub fn opposite_direction(mut self) -> Self {
        self.direction = self.direction.opposite();
        self
    }

    pub fn with_depth_movement(mut self, value: i8) -> Self {
        self.depth_movement_o = Some(value);
        self
    }

    pub fn without_depth_movement(mut self) -> Self {
        self.depth_movement_o = None;
        self
    }

    pub fn with_direction(mut self, value: MovementDirection) -> Self {
        self.direction = value;
        self
    }

    pub fn with_duration(mut self, value: f32) -> Self {
        self.duration = value;
        self
    }

    pub fn with_radius(mut self, value: f32) -> Self {
        self.radius = value;
        self
    }
}

#[derive(Clone, Copy, Debug)]
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

#[derive(Clone, Copy, Debug)]
pub struct LinearMovementEnemyStep {
    pub depth_movement_o: Option<i8>,
    pub direction: Vec2,
    pub trayectory: f32,
}

impl LinearMovementEnemyStep {
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

    pub fn with_direction(mut self, value: Vec2) -> Self {
        self.direction = value;
        self
    }

    pub fn with_trayectory(mut self, value: f32) -> Self {
        self.trayectory = value;
        self
    }

    pub fn with_depth_movement(mut self, value: i8) -> Self {
        self.depth_movement_o = Some(value);
        self
    }
}

#[derive(Clone, Copy, Debug)]
pub struct JumpEnemyStep {
    pub attacking: bool,
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
#[derive(Clone, Copy, Debug)]
pub enum EnemyStep {
    Attack(AttackEnemyStep),
    Circle(CircleAroundEnemyStep),
    Idle(IdleEnemyStep),
    LinearMovement(LinearMovementEnemyStep),
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
        self.get_duration_o()
            .unwrap_or_else(|| EnemyStep::max_duration())
    }

    pub fn get_duration_o(&self) -> Option<f32> {
        match self {
            EnemyStep::Attack(AttackEnemyStep { duration, .. }) => Some(*duration),
            EnemyStep::Circle(CircleAroundEnemyStep { duration, .. }) => Some(*duration),
            EnemyStep::Idle(IdleEnemyStep { duration, .. }) => Some(*duration),
            EnemyStep::LinearMovement { .. } => None,
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

    pub fn linear_movement_base() -> LinearMovementEnemyStep {
        LinearMovementEnemyStep::base()
    }
}

impl From<AttackEnemyStep> for EnemyStep {
    fn from(step: AttackEnemyStep) -> Self {
        EnemyStep::Attack(step)
    }
}

impl From<IdleEnemyStep> for EnemyStep {
    fn from(step: IdleEnemyStep) -> Self {
        EnemyStep::Idle(step)
    }
}

impl From<CircleAroundEnemyStep> for EnemyStep {
    fn from(step: CircleAroundEnemyStep) -> Self {
        EnemyStep::Circle(step)
    }
}

impl From<LinearMovementEnemyStep> for EnemyStep {
    fn from(step: LinearMovementEnemyStep) -> Self {
        EnemyStep::LinearMovement(step)
    }
}

impl From<JumpEnemyStep> for EnemyStep {
    fn from(step: JumpEnemyStep) -> Self {
        EnemyStep::Jump(step)
    }
}
