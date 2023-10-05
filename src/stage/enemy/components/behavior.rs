use std::collections::VecDeque;
use std::time::Duration;

use bevy::prelude::*;
use seldom_pixel::prelude::PxSubPosition;

use crate::{
    plugins::movement::linear::components::{
        LinearDirection, LinearMovementBundle, LinearSpeed, LinearTargetPosition,
        TargetingPositionX, TargetingPositionY,
    },
    stage::{
        data::{EnemyStep, GAME_BASE_SPEED},
        resources::StageTime,
    },
};

use super::{CircleAround, LinearMovement};

#[derive(Component, Clone, Debug)]
pub struct EnemyCurrentBehavior {
    pub started: Duration,
    pub behavior: EnemyStep,
}

#[derive(Component, Clone, Debug)]
pub enum BehaviorBundle {
    Idle(()),
    LinearMovement(
        (
            LinearMovement,
            LinearMovementBundle<StageTime, TargetingPositionX>,
            LinearMovementBundle<StageTime, TargetingPositionY>,
        ),
    ),
    Jump(()),
    Attack(()),
    Circle(CircleAround),
}

impl EnemyCurrentBehavior {
    pub fn get_bundles(
        &self,
        time_offset: Duration,
        current_position: &PxSubPosition,
        speed: f32,
    ) -> BehaviorBundle {
        match self.behavior {
            EnemyStep::Idle { .. } => BehaviorBundle::Idle(()),
            EnemyStep::LinearMovement {
                direction,
                trayectory,
            } => {
                let normalised_direction = direction.normalize();
                let velocity = normalised_direction * speed * GAME_BASE_SPEED;
                let coordinates = current_position.0 + normalised_direction * trayectory;
                BehaviorBundle::LinearMovement((
                    LinearMovement {
                        direction,
                        trayectory,
                    },
                    LinearMovementBundle::<StageTime, TargetingPositionX>::new(
                        current_position.0.x,
                        coordinates.x,
                        velocity.x,
                    ),
                    LinearMovementBundle::<StageTime, TargetingPositionY>::new(
                        current_position.0.y,
                        coordinates.y,
                        velocity.y,
                    ),
                ))
            }
            EnemyStep::Attack { .. } => BehaviorBundle::Attack(()),
            EnemyStep::Circle {
                radius, direction, ..
            } => BehaviorBundle::Circle(CircleAround {
                center: current_position.0,
                radius,
                direction: direction.clone(),
                time_offset: time_offset.as_secs_f32(),
            }),
            EnemyStep::Jump {
                coordinates,
                attacking,
                speed,
            } => BehaviorBundle::Jump(()),
        }
    }
}

#[derive(Component)]
pub struct EnemyBehaviors(pub VecDeque<EnemyStep>);

impl EnemyBehaviors {
    pub fn new(steps: VecDeque<EnemyStep>) -> Self {
        EnemyBehaviors(steps)
    }

    pub fn next(&mut self) -> EnemyStep {
        self.0.pop_front().unwrap_or_else(|| EnemyStep::Idle {
            duration: EnemyStep::max_duration(),
        })
    }
}

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
