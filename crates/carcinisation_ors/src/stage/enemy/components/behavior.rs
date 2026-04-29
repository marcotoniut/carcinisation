use super::{CircleAround, EnemyContinuousDepth, LinearTween};
use crate::stage::enemy::data::mosquito::MOSQUITO_DEPTH_RANGE;
use crate::stage::enemy::data::steps::{
    CircleAroundEnemyStep, EnemyStep, JumpEnemyStep, LinearTweenEnemyStep,
};
use crate::stage::{
    components::placement::Depth, data::GAME_BASE_SPEED, floors::ActiveFloors,
    resources::StageTimeDomain,
};
use bevy::prelude::*;
use carapace::prelude::WorldPos;
use cween::linear::components::{
    TargetingValueX, TargetingValueY, TargetingValueZ, TweenChildAcceleratedBundle,
    TweenChildBundle,
};

const MAX_JUMP_ARC_HEIGHT: f32 = 96.0;

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
#[allow(clippy::struct_excessive_bools)] // each bool tracks an independent axis-reached flag
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

#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct GroundedEnemyFall {
    pub vertical_velocity: f32,
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

fn jump_motion(
    step: JumpEnemyStep,
    current_position: &WorldPos,
    current_depth: EnemyContinuousDepth,
    gravity_acceleration: f32,
    target_y: f32,
) -> JumpMotion {
    let target_x = step.coordinates.x;
    let dx = target_x - current_position.0.x;
    let depth_value = current_depth.clamped_value();

    // Horizontal travel starts from authored speed, but absurdly long flights
    // would force absurd vertical launch speeds to still land on the floor.
    // Cap total airtime so jumps stay within a sane apex while still using
    // normal gravity and exact floor landing.
    let adapted_speed = (depth_value - 3.) / 6.;
    let base_x_speed = (step.speed + adapted_speed) * GAME_BASE_SPEED;
    let base_travel_time_secs = if base_x_speed.abs() > f32::EPSILON {
        dx.abs() / base_x_speed.abs()
    } else {
        1.0
    };

    let gravity_abs = gravity_acceleration.abs().max(f32::EPSILON);
    let apex_y = current_position.0.y.max(target_y) + MAX_JUMP_ARC_HEIGHT;
    let time_to_apex = ((2.0 * (apex_y - current_position.0.y).max(0.0)) / gravity_abs).sqrt();
    let time_from_apex = ((2.0 * (apex_y - target_y).max(0.0)) / gravity_abs).sqrt();
    let max_travel_time_secs = (time_to_apex + time_from_apex).max(f32::EPSILON);
    let travel_time_secs = base_travel_time_secs.min(max_travel_time_secs);
    let x_velocity = if dx.abs() > f32::EPSILON {
        dx / travel_time_secs
    } else {
        0.0
    };

    let gravity = -gravity_abs;
    let dy = target_y - current_position.0.y;
    let initial_y_velocity = (dy - 0.5 * gravity * travel_time_secs * travel_time_secs)
        / travel_time_secs.max(f32::EPSILON);

    let target_depth = step.depth_movement.map(|dm| {
        let clamped_dm = f32::from(dm.clamp(-2, 2));
        (depth_value + clamped_dm).clamp(
            MOSQUITO_DEPTH_RANGE.start().to_f32(),
            MOSQUITO_DEPTH_RANGE.end().to_f32(),
        )
    });
    let z_velocity = target_depth
        .map(|target_depth| (target_depth - depth_value) / travel_time_secs.max(f32::EPSILON));

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

#[must_use]
pub fn resolve_jump_target_depth(
    step: JumpEnemyStep,
    current_depth: EnemyContinuousDepth,
) -> Depth {
    let target_depth = step
        .depth_movement
        .map_or(current_depth.clamped_value(), |dm| {
            let clamped_dm = f32::from(dm.clamp(-2, 2));
            (current_depth.clamped_value() + clamped_dm).clamp(
                MOSQUITO_DEPTH_RANGE.start().to_f32(),
                MOSQUITO_DEPTH_RANGE.end().to_f32(),
            )
        });
    Depth::from_continuous(target_depth)
}

/// # Panics
/// Panics if the target depth has no solid floor.
#[must_use]
pub fn resolve_jump_target_y(
    step: JumpEnemyStep,
    current_depth: EnemyContinuousDepth,
    floors: &ActiveFloors,
    ground_anchor: f32,
) -> f32 {
    let target_depth = resolve_jump_target_depth(step, current_depth);
    let floor_y = floors
        .highest_solid_y(target_depth)
        .unwrap_or_else(|| panic!("jump target at depth {target_depth:?} requires a solid floor"));
    floor_y + ground_anchor
}

impl EnemyCurrentBehavior {
    /// # Panics
    /// Panics on `Jump` steps if `jump_target_y` is `None`.
    #[must_use]
    pub fn get_bundles(
        &self,
        time_offset: Duration,
        current_position: &WorldPos,
        _speed: f32,
        current_depth: EnemyContinuousDepth,
        gravity_acceleration: f32,
        jump_target_y: Option<f32>,
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
                let motion = jump_motion(
                    step,
                    current_position,
                    current_depth,
                    gravity_acceleration,
                    jump_target_y.expect("jump behaviors require a resolved floor target"),
                );
                BehaviorBundle::Jump(JumpTween::new(
                    time_offset,
                    motion.travel_time_secs,
                    motion.target_depth.is_some(),
                ))
            }
        }
    }

    /// Spawns tween child entities for movement behaviors (`LinearTween` and Jump).
    /// Returns a vector of child entity IDs.
    ///
    /// # Panics
    /// Panics on `Jump` steps if `jump_target_y` is `None`.
    #[allow(clippy::too_many_lines)]
    pub fn spawn_tween_children(
        &self,
        commands: &mut Commands,
        enemy_entity: Entity,
        current_position: &WorldPos,
        speed: f32,
        current_depth: EnemyContinuousDepth,
        gravity_acceleration: f32,
        jump_target_y: Option<f32>,
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
                let depth_value = current_depth.clamped_value();
                let adapted_speed = (depth_value - 3.) / 6.;
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
                    let target_depth = (depth_value + f32::from(depth_movement)).clamp(
                        MOSQUITO_DEPTH_RANGE.start().to_f32(),
                        MOSQUITO_DEPTH_RANGE.end().to_f32(),
                    );

                    let t = (target_position - current_position.0).length() / velocity.length();
                    let x = target_depth - depth_value;

                    let child_z = commands
                        .spawn((
                            TweenChildBundle::<StageTimeDomain, TargetingValueZ>::new(
                                enemy_entity,
                                depth_value,
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
                let motion = jump_motion(
                    step,
                    current_position,
                    current_depth,
                    gravity_acceleration,
                    jump_target_y.expect("jump behaviors require a resolved floor target"),
                );
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
                                current_depth.clamped_value(),
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
