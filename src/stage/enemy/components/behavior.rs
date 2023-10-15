use std::time::Duration;
use std::{collections::VecDeque, ops::Add};

use bevy::prelude::*;
use seldom_pixel::prelude::PxSubPosition;

use crate::stage::enemy::data::mosquito::{MOSQUITO_MAX_DEPTH, MOSQUITO_MIN_DEPTH};
use crate::{
    plugins::movement::linear::components::{
        LinearMovementBundle, TargetingPositionX, TargetingPositionY, TargetingPositionZ,
    },
    stage::{
        components::placement::Depth,
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
            Option<LinearMovementBundle<StageTime, TargetingPositionZ>>,
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
        depth: u8,
    ) -> BehaviorBundle {
        match self.behavior {
            EnemyStep::Idle { .. } => BehaviorBundle::Idle(()),
            EnemyStep::LinearMovement {
                detph_movement,
                direction,
                trayectory,
            } => {
                let normalised_direction = direction.normalize();
                let enemy_speed = speed * GAME_BASE_SPEED;
                let velocity = normalised_direction * enemy_speed;
                let target_position = current_position.0 + normalised_direction * trayectory;

                BehaviorBundle::LinearMovement((
                    LinearMovement {
                        direction,
                        trayectory,
                        reached_x: false,
                        reached_y: false,
                    },
                    LinearMovementBundle::<StageTime, TargetingPositionX>::new(
                        current_position.0.x,
                        target_position.x,
                        velocity.x,
                    ),
                    LinearMovementBundle::<StageTime, TargetingPositionY>::new(
                        current_position.0.y,
                        target_position.y,
                        velocity.y,
                    ),
                    detph_movement.map(|depth_movement| {
                        let target_depth = depth
                            .saturating_add_signed(depth_movement)
                            .min(MOSQUITO_MIN_DEPTH)
                            .max(MOSQUITO_MAX_DEPTH)
                            as f32;

                        LinearMovementBundle::<StageTime, TargetingPositionZ>::new(
                            depth as f32,
                            target_depth,
                            target_depth / enemy_speed,
                        )
                    }),
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
                attacking,
                coordinates,
                detph_movement,
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
