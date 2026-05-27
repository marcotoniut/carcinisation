#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
//! Legacy/composed mosquito parity tests.

mod common;

use carcinisation_base::direction::SpriteDirection;
use std::time::Duration;

use carcinisation::stage::enemy::{
    composed::ComposedAnimationState,
    data::mosquiton::{ACTION_IDLE_FLY, ACTION_SHOOT_FLY},
    mosquito::{
        entity::EnemyMosquitoAttacking,
        systems::{ENEMY_MOSQUITO_ATTACK_SPEED, ENEMY_MOSQUITO_RANGED_PRESENTATION},
    },
    mosquiton::entity::EnemyMosquitonAnimation,
};
use common::{
    advance_stage, build_attack_timing_app, spawn_composed_mosquiton, spawn_legacy_mosquito,
};

fn attack_cooldown() -> Duration {
    Duration::from_secs_f32(ENEMY_MOSQUITO_ATTACK_SPEED)
}

#[test]
fn legacy_and_composed_clear_attack_state_after_same_duration() {
    let mut app = build_attack_timing_app();
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
    let mut app = build_attack_timing_app();
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
        assert_eq!(
            state.requested_tag,
            SpriteDirection::Front.tag_name(ACTION_SHOOT_FLY)
        );
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
        assert_eq!(
            state.requested_tag,
            SpriteDirection::Front.tag_name(ACTION_IDLE_FLY)
        );
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
