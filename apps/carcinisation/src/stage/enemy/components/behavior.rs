use super::{CircleAround, LinearTween};
use crate::stage::components::placement::Depth;
use crate::stage::enemy::data::mosquito::MOSQUITO_DEPTH_RANGE;
use crate::stage::enemy::data::steps::{
    CircleAroundEnemyStep, EnemyStep, JumpEnemyStep, LinearTweenEnemyStep,
};
use crate::stage::{data::GAME_BASE_SPEED, resources::StageTimeDomain};
use bevy::prelude::*;
use carapace::prelude::PxSubPosition;
use cween::linear::components::{
    TargetingValueX, TargetingValueY, TargetingValueZ, TweenChildAcceleratedBundle,
    TweenChildBundle,
};
use derive_new::new;
use std::collections::VecDeque;
use std::time::Duration;

#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct EnemyCurrentBehavior {
    pub started: Duration,
    pub behavior: EnemyStep,
}

/// Marker component for tween children spawned for enemy linear tween steps.
#[derive(Component, Clone, Debug)]
pub struct EnemyStepTweenChild;

#[derive(Component, Clone, Debug)]
pub enum BehaviorBundle {
    Idle,
    /// `LinearTween` now returns just the `LinearTween` marker component.
    /// Movement children are spawned separately via Commands.
    LinearTween(LinearTween),
    Jump(JumpTween),
    Attack,
    Circle(CircleAround),
}

#[derive(Component, Clone, Copy, Debug, Reflect)]
#[reflect(Component)]
pub struct JumpTween {
    pub started: Duration,
    pub travel_time_secs: f32,
    pub reached_x: bool,
    pub reached_y: bool,
    pub reached_z: bool,
    pub expects_z: bool,
}

impl JumpTween {
    #[must_use]
    pub fn new(started: Duration, travel_time_secs: f32, expects_z: bool) -> Self {
        Self {
            started,
            travel_time_secs,
            reached_x: false,
            reached_y: false,
            reached_z: !expects_z,
            expects_z,
        }
    }

    #[must_use]
    pub fn progress_at(self, now: Duration) -> f32 {
        let elapsed = now.saturating_sub(self.started).as_secs_f32();
        elapsed / self.travel_time_secs.max(f32::EPSILON)
    }

    #[must_use]
    pub fn is_finished(self) -> bool {
        self.reached_x && self.reached_y && self.reached_z
    }
}

#[derive(Clone, Copy, Debug)]
struct JumpMotion {
    target_x: f32,
    target_y: f32,
    x_velocity: f32,
    initial_y_velocity: f32,
    gravity: f32,
    travel_time_secs: f32,
    target_depth: Option<f32>,
    z_velocity: Option<f32>,
}

fn jump_motion(step: JumpEnemyStep, current_position: &PxSubPosition, depth: Depth) -> JumpMotion {
    let target_x = step.coordinates.x;
    let target_y = step.coordinates.y;
    let dx = target_x - current_position.0.x;

    // Horizontal velocity derived from jump speed and GAME_BASE_SPEED.
    let adapted_speed = (depth.to_f32() - 3.) / 6.;
    let x_velocity = dx.signum() * (step.speed + adapted_speed) * GAME_BASE_SPEED;

    let travel_time_secs = if x_velocity.abs() > f32::EPSILON {
        dx.abs() / x_velocity.abs()
    } else {
        1.0
    };

    let gravity = -120.0;
    let dy = target_y - current_position.0.y;
    let initial_y_velocity = (dy - 0.5 * gravity * travel_time_secs * travel_time_secs)
        / travel_time_secs.max(f32::EPSILON);

    let target_depth = step.depth_movement.map(|dm| {
        let clamped_dm = dm.clamp(-2, 2);
        (depth + clamped_dm)
            .clamp(*MOSQUITO_DEPTH_RANGE.start(), *MOSQUITO_DEPTH_RANGE.end())
            .to_f32()
    });
    let z_velocity = target_depth
        .map(|target_depth| (target_depth - depth.to_f32()) / travel_time_secs.max(f32::EPSILON));

    JumpMotion {
        target_x,
        target_y,
        x_velocity,
        initial_y_velocity,
        gravity,
        travel_time_secs,
        target_depth,
        z_velocity,
    }
}

impl EnemyCurrentBehavior {
    #[must_use]
    pub fn get_bundles(
        &self,
        time_offset: Duration,
        current_position: &PxSubPosition,
        _speed: f32,
        depth: Depth,
    ) -> BehaviorBundle {
        match self.behavior {
            EnemyStep::Idle { .. } => BehaviorBundle::Idle,
            EnemyStep::LinearTween(LinearTweenEnemyStep {
                direction,
                trayectory,
                ..
            }) => BehaviorBundle::LinearTween(LinearTween {
                direction,
                trayectory,
                reached_x: false,
                reached_y: false,
            }),
            EnemyStep::Attack { .. } => BehaviorBundle::Attack,
            EnemyStep::Circle(CircleAroundEnemyStep {
                radius, direction, ..
            }) => BehaviorBundle::Circle(CircleAround {
                center: current_position.0,
                // TODO hardcoded values should be coming from the enemy type
                radius: radius.unwrap_or(12.),
                direction,
                time_offset: time_offset.as_secs_f32(),
            }),
            EnemyStep::Jump(step) => {
                let motion = jump_motion(step, current_position, depth);
                BehaviorBundle::Jump(JumpTween::new(
                    time_offset,
                    motion.travel_time_secs,
                    motion.target_depth.is_some(),
                ))
            }
        }
    }

    /// Spawns tween child entities for movement behaviors (LinearTween and Jump).
    /// Returns a vector of child entity IDs.
    pub fn spawn_tween_children(
        &self,
        commands: &mut Commands,
        enemy_entity: Entity,
        current_position: &PxSubPosition,
        speed: f32,
        depth: Depth,
    ) -> Vec<Entity> {
        let mut children = Vec::new();

        match self.behavior {
            EnemyStep::LinearTween(LinearTweenEnemyStep {
                depth_movement_o,
                direction,
                trayectory,
            }) => {
                let normalised_direction = direction.normalize_or_zero();
                // TODO use a better formula to increase speed for higher depths
                let adapted_speed = (depth.to_f32() - 3.) / 6.;
                let velocity = normalised_direction * (speed + adapted_speed) * GAME_BASE_SPEED;
                let target_position = current_position.0 + normalised_direction * trayectory;

                // Spawn X-axis tween child
                let child_x = commands
                    .spawn((
                        TweenChildBundle::<StageTimeDomain, TargetingValueX>::new(
                            enemy_entity,
                            current_position.0.x,
                            target_position.x,
                            velocity.x,
                        ),
                        EnemyStepTweenChild,
                        Name::new("Enemy Tween X"),
                    ))
                    .id();
                children.push(child_x);

                // Spawn Y-axis tween child
                let child_y = commands
                    .spawn((
                        TweenChildBundle::<StageTimeDomain, TargetingValueY>::new(
                            enemy_entity,
                            current_position.0.y,
                            target_position.y,
                            velocity.y,
                        ),
                        EnemyStepTweenChild,
                        Name::new("Enemy Tween Y"),
                    ))
                    .id();
                children.push(child_y);

                // Spawn Z-axis tween child if depth movement specified
                if let Some(depth_movement) = depth_movement_o {
                    let target_depth = depth + depth_movement;
                    let target_depth = target_depth
                        .clamp(*MOSQUITO_DEPTH_RANGE.start(), *MOSQUITO_DEPTH_RANGE.end())
                        .to_f32();

                    let t = (target_position - current_position.0).length() / velocity.length();
                    let x = target_depth - depth.to_f32();

                    let child_z = commands
                        .spawn((
                            TweenChildBundle::<StageTimeDomain, TargetingValueZ>::new(
                                enemy_entity,
                                depth.to_f32(),
                                target_depth,
                                x / t,
                            ),
                            EnemyStepTweenChild,
                            Name::new("Enemy Tween Z"),
                        ))
                        .id();
                    children.push(child_z);
                }
            }
            EnemyStep::Jump(step) => {
                let motion = jump_motion(step, current_position, depth);
                // Spawn X-axis tween child (linear, constant velocity).
                let child_x = commands
                    .spawn((
                        TweenChildBundle::<StageTimeDomain, TargetingValueX>::new(
                            enemy_entity,
                            current_position.0.x,
                            motion.target_x,
                            motion.x_velocity,
                        ),
                        EnemyStepTweenChild,
                        Name::new("Enemy Jump X"),
                    ))
                    .id();
                children.push(child_x);

                let child_y = commands
                    .spawn((
                        TweenChildAcceleratedBundle::<StageTimeDomain, TargetingValueY>::new(
                            enemy_entity,
                            current_position.0.y,
                            motion.target_y,
                            motion.initial_y_velocity,
                            motion.gravity,
                        ),
                        EnemyStepTweenChild,
                        Name::new("Enemy Jump Y"),
                    ))
                    .id();
                children.push(child_y);

                if let (Some(target_depth), Some(z_velocity)) =
                    (motion.target_depth, motion.z_velocity)
                {
                    let child_z = commands
                        .spawn((
                            TweenChildBundle::<StageTimeDomain, TargetingValueZ>::new(
                                enemy_entity,
                                depth.to_f32(),
                                target_depth,
                                z_velocity,
                            ),
                            EnemyStepTweenChild,
                            Name::new("Enemy Jump Z"),
                        ))
                        .id();
                    children.push(child_z);
                }
            }
            _ => {}
        }

        children
    }
}

#[derive(new, Component, Debug, Reflect)]
pub struct EnemyBehaviors(pub VecDeque<EnemyStep>);

impl EnemyBehaviors {
    pub fn next_step(&mut self) -> EnemyStep {
        self.0.pop_front().unwrap_or_default()
    }
}

#[derive(Component)]
pub struct EnemyBehaviorTimer {
    pub entity: Entity,
    pub timer: Timer,
}

impl EnemyBehaviorTimer {
    #[must_use]
    pub fn new(entity: Entity, duration: f32) -> Self {
        EnemyBehaviorTimer {
            entity,
            timer: Timer::from_seconds(duration, TimerMode::Once),
        }
    }
}
