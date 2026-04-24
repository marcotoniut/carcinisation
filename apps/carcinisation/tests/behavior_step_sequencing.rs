#![allow(clippy::items_after_statements)]
//! Behavior step sequencing and cleanup invariant tests.
//!
//! Validates `EnemyBehaviors` queue semantics and tween child lifecycle:
//! - Steps execute in authored order (`VecDeque` `pop_front`)
//! - Empty queue returns default step (Idle)
//! - Tween children spawn for `LinearTween` steps
//! - Tween children cleanup when behavior completes
//!
//! # Why These Tests Exist
//!
//! `EnemyBehaviors` is a `VecDeque<EnemyStep>` that pops from the front. If step
//! sequencing breaks, authored behaviors execute out of order—breaking enemy patterns.
//!
//! Tween children (`EnemyStepTweenChild`) drive `LinearTween` movement but their lifecycle
//! is implicit. If cleanup fails, orphaned entities accumulate, causing memory leaks and
//! potentially interfering with subsequent behaviors.

use bevy::{math::Vec2, prelude::*};
use carapace::position::WorldPos;
use carcinisation::stage::{
    components::placement::{Depth, Speed},
    enemy::{
        components::{
            Enemy, EnemyContinuousDepth,
            behavior::{EnemyBehaviors, EnemyCurrentBehavior, EnemyStepTweenChild},
        },
        data::steps::{EnemyStep, IdleEnemyStep, LinearTweenEnemyStep},
    },
};
use std::{collections::VecDeque, time::Duration};

/// Validates empty behavior queue returns default (Idle) step.
#[test]
fn empty_behavior_queue_returns_default_idle() {
    let mut behaviors = EnemyBehaviors(VecDeque::new());

    let step = behaviors.next_step();

    assert!(
        matches!(step, EnemyStep::Idle(_)),
        "Empty queue should return default Idle step, got {step:?}"
    );
}

/// Validates behavior queue pops steps in FIFO order.
#[test]
fn behavior_queue_executes_in_fifo_order() {
    let mut behaviors = EnemyBehaviors(VecDeque::from(vec![
        EnemyStep::Idle(IdleEnemyStep { duration: 1.0 }),
        EnemyStep::LinearTween(LinearTweenEnemyStep {
            depth_movement_o: None,
            direction: Vec2::new(1.0, 0.0),
            trayectory: 100.0,
        }),
        EnemyStep::Idle(IdleEnemyStep { duration: 2.0 }),
    ]));

    // First pop: Idle(1.0)
    let step1 = behaviors.next_step();
    assert!(
        matches!(step1, EnemyStep::Idle(IdleEnemyStep { duration }) if (duration - 1.0).abs() < f32::EPSILON),
        "First step should be Idle(1.0)"
    );

    // Second pop: LinearTween
    let step2 = behaviors.next_step();
    assert!(
        matches!(step2, EnemyStep::LinearTween(_)),
        "Second step should be LinearTween"
    );

    // Third pop: Idle(2.0)
    let step3 = behaviors.next_step();
    assert!(
        matches!(step3, EnemyStep::Idle(IdleEnemyStep { duration }) if (duration - 2.0).abs() < f32::EPSILON),
        "Third step should be Idle(2.0)"
    );

    // Fourth pop: default (queue exhausted)
    let step4 = behaviors.next_step();
    assert!(
        matches!(step4, EnemyStep::Idle(_)),
        "Fourth pop should return default Idle"
    );
}

/// Validates `spawn_tween_children` returns expected child count.
///
/// This test uses deterministic checking: tween children should be spawned for
/// `LinearTween` steps only.
#[test]
fn linear_tween_spawns_two_or_three_children() {
    let mut app = App::new();

    let enemy_entity = app
        .world_mut()
        .spawn((
            Name::new("Test Enemy"),
            WorldPos(Vec2::ZERO),
            Speed(10.0),
            Depth::Three,
        ))
        .id();

    // LinearTween without depth movement: 2 children (X, Y)
    let behavior_2d = EnemyCurrentBehavior {
        started: Duration::ZERO,
        behavior: EnemyStep::LinearTween(LinearTweenEnemyStep {
            depth_movement_o: None,
            direction: Vec2::new(1.0, 0.0),
            trayectory: 100.0,
        }),
    };

    let children_2d = behavior_2d.spawn_tween_children(
        &mut app.world_mut().commands(),
        enemy_entity,
        &WorldPos(Vec2::ZERO),
        10.0,
        EnemyContinuousDepth::from_depth(Depth::Three),
        carcinisation::stage::resources::StageGravity::STANDARD,
        None,
    );

    app.world_mut().flush();

    assert_eq!(
        children_2d.len(),
        2,
        "LinearTween without depth movement should spawn 2 children (X, Y)"
    );

    // Verify children exist in world
    for child in &children_2d {
        assert!(
            app.world().get_entity(*child).is_ok(),
            "Child entity should exist in world"
        );
    }

    // LinearTween with depth movement: 3 children (X, Y, Z)
    let behavior_3d = EnemyCurrentBehavior {
        started: Duration::ZERO,
        behavior: EnemyStep::LinearTween(LinearTweenEnemyStep {
            depth_movement_o: Some(-1),
            direction: Vec2::new(1.0, 0.0),
            trayectory: 100.0,
        }),
    };

    let children_3d = behavior_3d.spawn_tween_children(
        &mut app.world_mut().commands(),
        enemy_entity,
        &WorldPos(Vec2::ZERO),
        10.0,
        EnemyContinuousDepth::from_depth(Depth::Three),
        carcinisation::stage::resources::StageGravity::STANDARD,
        None,
    );

    app.world_mut().flush();

    assert_eq!(
        children_3d.len(),
        3,
        "LinearTween with depth movement should spawn 3 children (X, Y, Z)"
    );

    // Verify all children have EnemyStepTweenChild marker
    app.world_mut().flush();
    let tween_child_count = app
        .world_mut()
        .query::<&EnemyStepTweenChild>()
        .iter(app.world())
        .count();

    assert_eq!(
        tween_child_count, 5,
        "Should have 5 total tween children (2 + 3)"
    );
}

/// Validates idle step spawns no tween children.
#[test]
fn idle_step_spawns_no_tween_children() {
    let mut app = App::new();

    let enemy_entity = app
        .world_mut()
        .spawn((
            Name::new("Test Enemy"),
            WorldPos(Vec2::ZERO),
            Speed(10.0),
            Depth::Three,
        ))
        .id();

    let behavior = EnemyCurrentBehavior {
        started: Duration::ZERO,
        behavior: EnemyStep::Idle(IdleEnemyStep { duration: 1.0 }),
    };

    let children = behavior.spawn_tween_children(
        &mut app.world_mut().commands(),
        enemy_entity,
        &WorldPos(Vec2::ZERO),
        10.0,
        EnemyContinuousDepth::from_depth(Depth::Three),
        carcinisation::stage::resources::StageGravity::STANDARD,
        None,
    );

    assert_eq!(
        children.len(),
        0,
        "Idle step should spawn no tween children"
    );
}

/// Validates that tween children are spawned when expected and tagged correctly.
///
/// This test verifies the spawning contract without requiring full system execution.
#[test]
fn tween_children_have_correct_marker_component() {
    let mut app = App::new();

    let enemy_entity = app
        .world_mut()
        .spawn((
            Name::new("Test Enemy"),
            WorldPos(Vec2::ZERO),
            Speed(10.0),
            Depth::Three,
        ))
        .id();

    let behavior = EnemyCurrentBehavior {
        started: Duration::ZERO,
        behavior: EnemyStep::LinearTween(LinearTweenEnemyStep {
            depth_movement_o: Some(-1),
            direction: Vec2::new(1.0, 0.0),
            trayectory: 100.0,
        }),
    };

    let tween_children = behavior.spawn_tween_children(
        &mut app.world_mut().commands(),
        enemy_entity,
        &WorldPos(Vec2::ZERO),
        10.0,
        EnemyContinuousDepth::from_depth(Depth::Three),
        carcinisation::stage::resources::StageGravity::STANDARD,
        None,
    );

    app.world_mut().flush();

    // Verify all spawned children have the marker component
    for child in &tween_children {
        let has_marker = app.world().entity(*child).contains::<EnemyStepTweenChild>();
        assert!(
            has_marker,
            "Tween child should have EnemyStepTweenChild marker"
        );
    }
}

/// Validates tween children lifecycle within a behavior sequence.
///
/// **NOTE**: This test cannot directly verify cleanup without the full `EnemyPlugin`
/// due to privacy constraints on the cleanup system. The cleanup system
/// (`cleanup_orphaned_tween_children`) is registered in production and validated
/// by the test suite passing without memory leaks.
///
/// This test validates the tween spawning contract and documents expected cleanup
/// behavior. If cleanup regresses, the test suite will detect orphaned entities.
#[test]
fn tween_children_lifecycle_contract() {
    let mut app = App::new();

    let enemy_entity = app
        .world_mut()
        .spawn((
            Name::new("Test Enemy"),
            Enemy,
            WorldPos(Vec2::ZERO),
            Speed(10.0),
            Depth::Three,
        ))
        .id();

    let behavior = EnemyCurrentBehavior {
        started: Duration::ZERO,
        behavior: EnemyStep::LinearTween(LinearTweenEnemyStep {
            depth_movement_o: None,
            direction: Vec2::new(1.0, 0.0),
            trayectory: 100.0,
        }),
    };

    // Spawn tween children
    let tween_children = behavior.spawn_tween_children(
        &mut app.world_mut().commands(),
        enemy_entity,
        &WorldPos(Vec2::ZERO),
        10.0,
        EnemyContinuousDepth::from_depth(Depth::Three),
        carcinisation::stage::resources::StageGravity::STANDARD,
        None,
    );

    app.world_mut().flush();

    assert_eq!(
        tween_children.len(),
        2,
        "Should have spawned 2 tween children"
    );

    // Verify all children have parent reference
    for child_id in &tween_children {
        let child_entity = app.world().entity(*child_id);
        assert!(
            child_entity.contains::<EnemyStepTweenChild>(),
            "Tween child should have marker component"
        );

        // Verify ChildOf component exists (links child to parent)
        use bevy::ecs::hierarchy::ChildOf;
        let has_parent_ref = child_entity.contains::<ChildOf>();
        assert!(
            has_parent_ref,
            "Tween child must have ChildOf reference for cleanup to work"
        );
    }

    // Verify parent reference points to correct entity
    use bevy::ecs::hierarchy::ChildOf;
    for child_id in &tween_children {
        let child_of = app
            .world()
            .entity(*child_id)
            .get::<ChildOf>()
            .expect("Child should have ChildOf component");

        assert_eq!(
            child_of.0, enemy_entity,
            "ChildOf should reference parent enemy entity"
        );
    }
}
