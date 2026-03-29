//! Behavior step variant validation.
//!
//! Validates that all EnemyStep enum variants are handled correctly in queue
//! drain operations by testing a sequence containing every variant type.

use bevy::math::Vec2;
use carcinisation::stage::enemy::{
    components::behavior::EnemyBehaviors,
    data::steps::{
        AttackEnemyStep, CircleAroundEnemyStep, EnemyStep, IdleEnemyStep, JumpEnemyStep,
        LinearTweenEnemyStep,
    },
};
use cween::structs::TweenDirection;
use std::collections::VecDeque;

/// Validates mixed behavior types maintain correct FIFO ordering.
///
/// Golden sequence covering all step variants should pop in authored order.
#[test]
fn mixed_behavior_types_golden_sequence() {
    let mut behaviors = EnemyBehaviors(VecDeque::from(vec![
        EnemyStep::Idle(IdleEnemyStep { duration: 1.0 }),
        EnemyStep::LinearTween(LinearTweenEnemyStep {
            depth_movement_o: Some(-1),
            direction: Vec2::new(1.0, 0.0),
            trayectory: 50.0,
        }),
        EnemyStep::Circle(CircleAroundEnemyStep {
            depth_movement_o: None,
            radius: Some(15.0),
            duration: Some(2.0),
            direction: TweenDirection::Positive,
        }),
        EnemyStep::Jump(JumpEnemyStep {
            attacking: false,
            coordinates: Vec2::new(20.0, 0.0),
            depth_movement: None,
            speed: 0.5,
        }),
        EnemyStep::Attack(AttackEnemyStep { duration: 1.5 }),
    ]));

    let expected_sequence = vec!["Idle", "LinearTween", "Circle", "Jump", "Attack"];
    let mut actual_sequence = Vec::new();

    for _ in 0..5 {
        let step = behaviors.next_step();
        let variant = match step {
            EnemyStep::Idle(_) => "Idle",
            EnemyStep::LinearTween(_) => "LinearTween",
            EnemyStep::Circle(_) => "Circle",
            EnemyStep::Attack { .. } => "Attack",
            EnemyStep::Jump(_) => "Jump",
        };
        actual_sequence.push(variant);
    }

    assert_eq!(
        actual_sequence, expected_sequence,
        "Mixed behavior sequence should match golden order"
    );
}
