use super::{CircleAround, LinearTween};
use crate::stage::components::placement::Depth;
use crate::stage::enemy::data::mosquito::MOSQUITO_DEPTH_RANGE;
use crate::stage::enemy::data::steps::{
    CircleAroundEnemyStep, EnemyStep, JumpEnemyStep, LinearTweenEnemyStep,
};
use crate::stage::{data::GAME_BASE_SPEED, resources::StageTimeDomain};
use bevy::prelude::*;
use cween::linear::components::{
    TargetingValueX, TargetingValueY, TargetingValueZ, TweenChildBundle,
};
use derive_new::new;
use seldom_pixel::prelude::PxSubPosition;
use std::collections::VecDeque;
use std::time::Duration;

#[derive(Component, Clone, Debug, Reflect)]
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
    /// LinearTween now returns just the LinearTween marker component.
    /// Movement children are spawned separately via Commands.
    LinearTween(LinearTween),
    Jump,
    Attack,
    Circle(CircleAround),
}

impl EnemyCurrentBehavior {
    pub fn get_bundles(
        &self,
        time_offset: Duration,
        current_position: &PxSubPosition,
        _speed: f32,
        _depth: Depth,
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
            EnemyStep::Jump(JumpEnemyStep { .. }) => BehaviorBundle::Jump,
        }
    }

    /// Spawns tween child entities for linear tween behaviors.
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

        if let EnemyStep::LinearTween(LinearTweenEnemyStep {
            depth_movement_o,
            direction,
            trayectory,
        }) = self.behavior
        {
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
    pub fn new(entity: Entity, duration: f32) -> Self {
        EnemyBehaviorTimer {
            entity,
            timer: Timer::from_seconds(duration, TimerMode::Once),
        }
    }
}
