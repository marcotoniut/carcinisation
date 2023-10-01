use std::collections::VecDeque;
use std::time::Duration;

use bevy::prelude::*;
use seldom_pixel::prelude::PxSubPosition;

use crate::stage::data::EnemyStep;

use super::{CircleAround, LinearMovement};

#[derive(Component, Clone, Debug)]
pub struct EnemyCurrentBehavior {
    pub started: Duration,
    pub behavior: EnemyStep,
}

#[derive(Component, Clone, Debug)]
pub enum BehaviorBundle {
    Idle(()),
    LinearMovement(LinearMovement),
    Jump(()),
    Attack(()),
    Circle(CircleAround),
}

impl EnemyCurrentBehavior {
    pub fn get_bundles(
        &self,
        time_offset: Duration,
        current_position: &PxSubPosition,
    ) -> BehaviorBundle {
        match self.behavior {
            EnemyStep::Idle { .. } => BehaviorBundle::Idle(()),
            EnemyStep::LinearMovement {
                coordinates,
                attacking,
                speed,
            } => BehaviorBundle::LinearMovement(LinearMovement {
                direction: coordinates - current_position.0,
                trayectory: coordinates.distance(current_position.0),
            }),
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
