use super::{CircleAround, LinearMovement};
use crate::stage::components::placement::Depth;
use crate::stage::enemy::data::mosquito::{MOSQUITO_MAX_DEPTH, MOSQUITO_MIN_DEPTH};
use crate::stage::enemy::data::steps::{
    CircleAroundEnemyStep, EnemyStep, JumpEnemyStep, LinearMovementEnemyStep,
};
use crate::{
    plugins::movement::linear::components::{
        LinearMovementBundle, TargetingPositionX, TargetingPositionY, TargetingPositionZ,
    },
    stage::{data::GAME_BASE_SPEED, resources::StageTime},
};
use bevy::prelude::*;
use seldom_pixel::prelude::PxSubPosition;
use std::collections::VecDeque;
use std::time::Duration;

#[derive(Component, Clone, Debug, Reflect)]
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
        depth: Depth,
    ) -> BehaviorBundle {
        match self.behavior {
            EnemyStep::Idle { .. } => BehaviorBundle::Idle(()),
            EnemyStep::LinearMovement(LinearMovementEnemyStep {
                depth_movement_o,
                direction,
                trayectory,
            }) => {
                let normalised_direction = direction.normalize_or_zero();
                // TODO use a better formula to increase speed for higher depths
                let adapted_speed = (depth.to_f32() - 3.) / 6.;
                let velocity = normalised_direction * (speed + adapted_speed) * GAME_BASE_SPEED;
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
                    depth_movement_o.map(|depth_movement| {
                        let target_depth = depth + depth_movement;
                        let target_depth = target_depth
                            .min(MOSQUITO_MAX_DEPTH)
                            .max(MOSQUITO_MIN_DEPTH)
                            .to_f32();

                        let t = (target_position - current_position.0).length() / velocity.length();
                        let x = target_depth - depth.to_f32();

                        LinearMovementBundle::<StageTime, TargetingPositionZ>::new(
                            depth.to_f32(),
                            target_depth,
                            // REVIEW extra multiplier
                            x / t,
                        )
                    }),
                ))
            }
            EnemyStep::Attack { .. } => BehaviorBundle::Attack(()),
            EnemyStep::Circle(CircleAroundEnemyStep {
                radius, direction, ..
            }) => BehaviorBundle::Circle(CircleAround {
                center: current_position.0,
                radius,
                direction: direction.clone(),
                time_offset: time_offset.as_secs_f32(),
            }),
            EnemyStep::Jump(JumpEnemyStep {
                attacking,
                coordinates,
                depth_movement,
                speed,
            }) => BehaviorBundle::Jump(()),
        }
    }
}

#[derive(Component, Reflect)]
pub struct EnemyBehaviors(pub VecDeque<EnemyStep>);

impl EnemyBehaviors {
    pub fn new(steps: VecDeque<EnemyStep>) -> Self {
        EnemyBehaviors(steps)
    }

    pub fn next(&mut self) -> EnemyStep {
        self.0.pop_front().unwrap_or_else(|| EnemyStep::default())
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
