#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
//! Legacy/composed mosquito parity tests.

use std::time::Duration;

use bevy::prelude::*;
use carcinisation::stage::{
    components::placement::Depth,
    enemy::{
        components::behavior::EnemyCurrentBehavior,
        composed::ComposedAnimationState,
        data::{
            mosquiton::{TAG_IDLE_FLY, TAG_SHOOT_FLY},
            steps::{EnemyStep, IdleEnemyStep},
        },
        mosquito::{
            entity::{EnemyMosquito, EnemyMosquitoAttack, EnemyMosquitoAttacking},
            systems::{
                ENEMY_MOSQUITO_ATTACK_SPEED, ENEMY_MOSQUITO_RANGED_PRESENTATION,
                clear_finished_mosquito_attacks,
            },
        },
        mosquiton::{
            entity::{EnemyMosquiton, EnemyMosquitonAnimation},
            systems::assign_mosquiton_animation,
        },
    },
    resources::StageTimeDomain,
};

fn attack_cooldown() -> Duration {
    Duration::from_secs_f32(ENEMY_MOSQUITO_ATTACK_SPEED)
}

fn build_attack_test_app() -> App {
    let mut app = App::new();
    app.insert_resource(Time::<StageTimeDomain>::default());
    app.add_systems(
        Update,
        (clear_finished_mosquito_attacks, assign_mosquiton_animation).chain(),
    );
    app
}

fn spawn_legacy_mosquito(app: &mut App) -> Entity {
    app.world_mut()
        .spawn((
            EnemyMosquito,
            EnemyCurrentBehavior {
                started: Duration::ZERO,
                behavior: EnemyStep::Idle(IdleEnemyStep { duration: 99999.0 }),
            },
            EnemyMosquitoAttacking {
                attack: Some(EnemyMosquitoAttack::Ranged),
                last_attack_started: Duration::ZERO,
            },
            Depth::Three,
        ))
        .id()
}

fn spawn_composed_mosquiton(app: &mut App) -> Entity {
    app.world_mut()
        .spawn((
            EnemyMosquiton,
            EnemyMosquito,
            EnemyCurrentBehavior {
                started: Duration::ZERO,
                behavior: EnemyStep::Idle(IdleEnemyStep { duration: 99999.0 }),
            },
            EnemyMosquitoAttacking {
                attack: Some(EnemyMosquitoAttack::Ranged),
                last_attack_started: Duration::ZERO,
            },
            ComposedAnimationState::new(TAG_IDLE_FLY),
            Depth::Three,
        ))
        .id()
}

fn advance_stage(app: &mut App, duration: Duration) {
    app.world_mut()
        .resource_mut::<Time<StageTimeDomain>>()
        .advance_by(duration);
    app.update();
}

#[test]
fn legacy_and_composed_clear_attack_state_after_same_duration() {
    let mut app = build_attack_test_app();
    let legacy = spawn_legacy_mosquito(&mut app);
    let composed = spawn_composed_mosquiton(&mut app);

    app.update();

    advance_stage(
        &mut app,
        ENEMY_MOSQUITO_RANGED_PRESENTATION + Duration::from_millis(1),
    );

    assert!(
        app.world()
            .entity(legacy)
            .get::<EnemyMosquitoAttacking>()
            .unwrap()
            .attack
            .is_none()
    );
    assert!(
        app.world()
            .entity(composed)
            .get::<EnemyMosquitoAttacking>()
            .unwrap()
            .attack
            .is_none()
    );
}

#[test]
fn composed_mosquiton_requests_shoot_then_returns_to_idle() {
    let mut app = build_attack_test_app();
    let composed = spawn_composed_mosquiton(&mut app);

    app.update();
    {
        let animation = app
            .world()
            .entity(composed)
            .get::<EnemyMosquitonAnimation>()
            .expect("composed animation should be assigned");
        let state = app
            .world()
            .entity(composed)
            .get::<ComposedAnimationState>()
            .expect("composed state should exist");
        assert_eq!(*animation, EnemyMosquitonAnimation::ShootFly);
        assert_eq!(state.requested_tag, TAG_SHOOT_FLY);
    }

    advance_stage(
        &mut app,
        ENEMY_MOSQUITO_RANGED_PRESENTATION + Duration::from_millis(1),
    );

    {
        let animation = app
            .world()
            .entity(composed)
            .get::<EnemyMosquitonAnimation>()
            .expect("composed animation should still be assigned");
        let state = app
            .world()
            .entity(composed)
            .get::<ComposedAnimationState>()
            .expect("composed state should exist");
        assert_eq!(*animation, EnemyMosquitonAnimation::IdleFly);
        assert_eq!(state.requested_tag, TAG_IDLE_FLY);
    }
}

#[test]
fn presentation_window_stays_shorter_than_shared_attack_cooldown() {
    let cooldown = attack_cooldown();
    let attack_budget =
        (Duration::from_secs(15).as_secs_f32() / cooldown.as_secs_f32()).ceil() as u64;

    assert!(ENEMY_MOSQUITO_RANGED_PRESENTATION < cooldown);
    assert!(
        (4..=5).contains(&attack_budget),
        "15 seconds should still budget roughly 4-5 attacks at the shared cooldown"
    );
}
