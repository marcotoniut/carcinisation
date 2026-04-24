//! Thin mosquiton pattern constructors.
//!
//! These return plain `VecDeque<EnemyStep>` chunks for higher-level decision
//! code to enqueue. They do not execute behavior themselves.

use crate::stage::{components::placement::Depth, enemy::data::steps::EnemyStep};
use cween::structs::TweenDirection;
use std::collections::VecDeque;

/// Builds the simple "advance into the ranged attack band" pattern.
#[must_use]
pub fn mosquiton_approach_to_band() -> VecDeque<EnemyStep> {
    VecDeque::from([EnemyStep::LinearTween(
        EnemyStep::linear_movement_base()
            .depth_advance(1)
            .with_direction(-1.0, -0.25)
            .with_trayectory(28.0),
    )])
}

/// Builds the simple "hold position long enough to shoot" pattern.
#[must_use]
pub fn mosquiton_hold_and_shoot(depth: Depth) -> VecDeque<EnemyStep> {
    let direction = if depth.to_i8() % 2 == 0 {
        TweenDirection::Positive
    } else {
        TweenDirection::Negative
    };

    VecDeque::from([EnemyStep::Circle(
        EnemyStep::circle_around_base()
            .without_depth_movement()
            .with_direction(direction)
            .with_duration(4.0)
            .with_radius(7.0),
    )])
}

/// Builds the simple "back out of the attack band" pattern.
#[must_use]
pub fn mosquiton_retreat(depth: Depth) -> VecDeque<EnemyStep> {
    let direction = if depth.to_i8() % 2 == 0 {
        (1.0, 0.25)
    } else {
        (-1.0, 0.25)
    };

    VecDeque::from([EnemyStep::LinearTween(
        EnemyStep::linear_movement_base()
            .depth_retreat(1)
            .with_direction(direction.0, direction.1)
            .with_trayectory(22.0),
    )])
}
